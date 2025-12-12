use std::io::{Read, Seek, Write};

use dicom_core::value::Value as DicomValue;
use dicom_dictionary_std::tags as dicom_tags;
use tiff::encoder::TiffKind;
use tiff::encoder::{TiffEncoder, TiffKindBig};
use tiff::tags::{PhotometricInterpretation as TiffPhotometricInterpretation, Tag as TiffTag};

mod shared_read_seek;
use shared_read_seek::SharedReadSeek;

type BoxErrorResult<T> = Result<T, Box<dyn std::error::Error>>;

fn get_dicom_pyramid_sources(sources: Vec<SharedReadSeek>) -> BoxErrorResult<Vec<SharedReadSeek>> {
    let mut dcm_objects = Vec::new();
    for source in sources {
        let obj = dicom_object::OpenFileOptions::new()
            .read_until(dicom_tags::PIXEL_DATA)
            .from_reader(source.clone())?;
        let image_type = obj.element(dicom_tags::IMAGE_TYPE)?.to_multi_str()?;
        // Only take pyramid levels
        let vals: Vec<&str> = image_type.iter().map(|s| s.trim()).collect();
        let v1 = ["ORIGINAL", "PRIMARY", "VOLUME", "NONE"];
        let v2 = ["DERIVED", "PRIMARY", "VOLUME", "NONE"];
        let v3 = ["DERIVED", "PRIMARY", "VOLUME", "RESAMPLED"];
        if vals == v1 || vals == v2 || vals == v3 {
            dcm_objects.push((source, obj));
        }
    }

    // Sort descending by TOTAL_PIXEL_MATRIX_COLUMNS, so pyramid levels are in order from 0 up.
    dcm_objects.sort_by(|a, b| {
        let a_cols =
            a.1.element(dicom_tags::TOTAL_PIXEL_MATRIX_COLUMNS)
                .ok()
                .and_then(|e| e.uint32().ok())
                .unwrap_or(0);
        let b_cols =
            b.1.element(dicom_tags::TOTAL_PIXEL_MATRIX_COLUMNS)
                .ok()
                .and_then(|e| e.uint32().ok())
                .unwrap_or(0);
        b_cols.cmp(&a_cols)
    });

    let sources = dcm_objects
        .into_iter()
        .map(|(mut source, _)| {
            source.rewind()?;
            Ok(source)
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

    Ok(sources)
}

fn dicom_photometric_interpretation_to_tiff(
    dcm_photometric_interpretation: &str,
) -> BoxErrorResult<(TiffPhotometricInterpretation, Option<[u16; 2]>)> {
    match dcm_photometric_interpretation {
        "MONOCHROME1" => Ok((TiffPhotometricInterpretation::BlackIsZero, None)),
        "MONOCHROME2" => Ok((TiffPhotometricInterpretation::WhiteIsZero, None)),
        "RGB" => Ok((TiffPhotometricInterpretation::RGB, None)),
        "YBR_FULL" => Ok((TiffPhotometricInterpretation::YCbCr, Some([1, 1]))),
        "YBR_FULL_422" => Ok((TiffPhotometricInterpretation::YCbCr, Some([2, 1]))),
        "YBR_ICT" => Ok((TiffPhotometricInterpretation::YCbCr, None)),
        _ => Err(format!(
            "Unsupported photometric interpretation: {}",
            dcm_photometric_interpretation
        )
        .into()),
    }
}

pub fn convert_dicom_sources<R: Read + Seek, W: Write + Seek>(
    dicom_sources: Vec<R>,
    output: W,
) -> BoxErrorResult<()> {
    let dicom_read_seeks = dicom_sources
        .into_iter()
        .map(|r| SharedReadSeek::from_read_seek(r))
        .collect::<Vec<_>>();
    let dicom_pyramid_sources = get_dicom_pyramid_sources(dicom_read_seeks)?;
    if dicom_pyramid_sources.is_empty() {
        return Err("No pyramid levels found".into());
    }

    let mut tiff = TiffEncoder::new_big(output)?;

    for dcm_source in dicom_pyramid_sources {
        let dcm_object = dicom_object::from_reader(dcm_source)?;
        // TODO: Handle sparsely tiled DICOMs
        let is_sparse = dcm_object
            .element_opt(dicom_tags::DIMENSION_ORGANIZATION_TYPE)?
            .and_then(|e| e.to_str().ok())
            == Some("TILED_SPARSE".into());
        if is_sparse {
            return Err("Sparsely tiled images are not supported".into());
        }

        let image_height = dcm_object
            .element(dicom_tags::TOTAL_PIXEL_MATRIX_ROWS)?
            .uint32()?;
        let image_width = dcm_object
            .element(dicom_tags::TOTAL_PIXEL_MATRIX_COLUMNS)?
            .uint32()?;
        let tile_height = dcm_object.element(dicom_tags::ROWS)?.uint16()?;
        let tile_width = dcm_object.element(dicom_tags::COLUMNS)?.uint16()?;
        let dcm_photometric_interpretation = dcm_object
            .element(dicom_tags::PHOTOMETRIC_INTERPRETATION)?
            .to_str()?;
        let (tiff_photometric_interpretation, subsampling) =
            dicom_photometric_interpretation_to_tiff(&dcm_photometric_interpretation)?;
        let samples_per_pixel = dcm_object
            .element(dicom_tags::SAMPLES_PER_PIXEL)?
            .uint16()?;
        let shared_functional_groups_sequence = match dcm_object
            .element(dicom_tags::SHARED_FUNCTIONAL_GROUPS_SEQUENCE)?
            .clone()
            .into_value()
        {
            DicomValue::Sequence(seq) => seq,
            _ => return Err("Expected SHARED_FUNCTIONAL_GROUPS_SEQUENCE to be a sequence".into()),
        };
        let shared_functional_groups_items = shared_functional_groups_sequence.into_items();
        if shared_functional_groups_items.is_empty() {
            return Err("SHARED_FUNCTIONAL_GROUPS_SEQUENCE is empty".into());
        }
        let first_shared_functional_group = &shared_functional_groups_items[0];
        let pixel_measures_sequence = match first_shared_functional_group
            .element(dicom_tags::PIXEL_MEASURES_SEQUENCE)?
            .clone()
            .into_value()
        {
            DicomValue::Sequence(seq) => seq,
            _ => return Err("Expected PIXEL_MEASURES_SEQUENCE to be a sequence".into()),
        };
        let pixel_measures_items = pixel_measures_sequence.into_items();
        if pixel_measures_items.is_empty() {
            return Err("PIXEL_MEASURES_SEQUENCE is empty".into());
        }
        let first_pixel_measures = &pixel_measures_items[0];
        let pixel_spacing_strs = first_pixel_measures
            .element(dicom_tags::PIXEL_SPACING)?
            .strings()?;
        let pixel_spacing = pixel_spacing_strs
            .iter()
            .map(|s| s.trim().parse::<f64>())
            .collect::<Result<Vec<_>, _>>()?;
        if pixel_spacing.len() != 2 {
            return Err("Expected PIXEL_SPACING to have 2 values".into());
        }
        let pixel_spacing_x = pixel_spacing[0];
        let pixel_spacing_y = pixel_spacing[1];
        let mpp_x = pixel_spacing_x * 1000.0;
        let mpp_y = pixel_spacing_y * 1000.0;
        // Centimeters
        let x_resolution = 10000.0 / mpp_x;
        let y_resolution = 10000.0 / mpp_y;

        let bits_stored = dcm_object.element(dicom_tags::BITS_STORED)?.uint16()?;
        let bits_per_sample = vec![bits_stored; samples_per_pixel as usize];

        // TODO: What if not lossy (e.g. LZW or not compressed, etc)?
        let lossy_compression_method = dcm_object
            .element(dicom_tags::LOSSY_IMAGE_COMPRESSION_METHOD)?
            .string()?
            .trim();
        let tiff_compression = match (lossy_compression_method, tiff_photometric_interpretation) {
            // ("ISO_10918_1", _) => tiff::tags::CompressionMethod::JPEG,
            ("ISO_10918_1", _) => tiff::tags::CompressionMethod::ModernJPEG,
            // ("ISO_15444_1", _) => tiff::tags::CompressionMethod::Unknown(34712),
            // APERIO_COMPRESSION_JP2K_RGB
            ("ISO_15444_1", TiffPhotometricInterpretation::RGB) => {
                tiff::tags::CompressionMethod::Unknown(33005)
            }
            // APERIO_COMPRESSION_JP2K_YCBCR
            ("ISO_15444_1", TiffPhotometricInterpretation::YCbCr) => {
                tiff::tags::CompressionMethod::Unknown(33003)
            }
            _ => {
                return Err("Unsupported lossy compression method".into());
            }
        };

        let optical_path_sequence = dcm_object
            .element(dicom_tags::OPTICAL_PATH_SEQUENCE)?
            .clone()
            .into_value();
        let optical_path_items = match optical_path_sequence {
            DicomValue::Sequence(seq) => seq.into_items(),
            _ => return Err("Expected OPTICAL_PATH_SEQUENCE to be a sequence".into()),
        };
        if optical_path_items.is_empty() {
            return Err("OPTICAL_PATH_SEQUENCE is empty".into());
        }
        let icc_profile_item = optical_path_items.iter().find(|item| {
            item.element_opt(dicom_tags::ICC_PROFILE)
                .ok()
                .flatten()
                .is_some()
        });
        let icc_profile = icc_profile_item.map(|item| {
            let elem = item.element(dicom_tags::ICC_PROFILE).unwrap();
            let value = elem.clone().into_value();
            let bytes = value.to_bytes().unwrap();
            bytes.to_vec()
        });

        let pixel_data = dcm_object
            .element(dicom_tags::PIXEL_DATA)?
            .fragments()
            .ok_or("PIXEL_DATA is of wrong type")?;

        let mut dir = tiff.image_directory()?;

        // Fake Aperio SVS
        let image_description = format!("Aperio\n|MPP={}", mpp_x);
        dir.write_tag(TiffTag::ImageDescription, image_description.as_str())?;

        // Dimensions
        dir.write_tag(TiffTag::ImageWidth, image_width)?;
        dir.write_tag(TiffTag::ImageLength, image_height)?;
        dir.write_tag(TiffTag::TileWidth, tile_width)?;
        dir.write_tag(TiffTag::TileLength, tile_height)?;
        // Resolution (MPP)
        dir.write_tag(TiffTag::ResolutionUnit, 3)?; // 3 = centimeters
        dir.write_tag(TiffTag::XResolution, x_resolution)?;
        dir.write_tag(TiffTag::YResolution, y_resolution)?;
        // Image related
        dir.write_tag(
            TiffTag::PhotometricInterpretation,
            tiff_photometric_interpretation.to_u16(),
        )?;
        // Tag: YCbCrSubSampling
        let ycbcr_subsampling_tag = TiffTag::Unknown(530);
        if let Some(subsampling) = subsampling {
            dir.write_tag(ycbcr_subsampling_tag, &subsampling[..])?;
        }
        dir.write_tag(TiffTag::SamplesPerPixel, samples_per_pixel)?;
        dir.write_tag(TiffTag::BitsPerSample, &bits_per_sample[..])?;

        dir.write_tag(TiffTag::Compression, tiff_compression.to_u16())?;

        // TODO: If compression is JPEG, extract the JPEG tables from the pixel data
        // (checking that they all are the same, which they should be) and set the
        // `TiffTag::JpegTables` tag with the shared JPEG tables.

        if let Some(icc_profile) = icc_profile {
            dir.write_tag(TiffTag::IccProfile, &icc_profile[..])?;
        }

        // Image Data
        let mut offsets = Vec::with_capacity(pixel_data.len());
        let mut byte_counts = Vec::with_capacity(pixel_data.len());
        for tile in pixel_data {
            let byte_count = tile.len() as u64;
            let offset = dir.write_data(&tile[..])?;
            offsets.push(TiffKindBig::convert_offset(offset)?);
            // let byte_count = dir.last_written();
            byte_counts.push(TiffKindBig::convert_offset(byte_count)?);
        }
        dir.write_tag(TiffTag::TileOffsets, TiffKindBig::convert_slice(&offsets))?;
        dir.write_tag(
            TiffTag::TileByteCounts,
            TiffKindBig::convert_slice(&byte_counts),
        )?;

        dir.finish()?;
    }

    Ok(())
}
