use std::io::{BufReader, Read, Seek, Write};

use wasm_bindgen::prelude::*;

mod panic_hook;

struct FileSystemSyncAccessHandleWrapper {
    pos: u64,
    handle: web_sys::FileSystemSyncAccessHandle,
}

impl From<web_sys::FileSystemSyncAccessHandle> for FileSystemSyncAccessHandleWrapper {
    fn from(handle: web_sys::FileSystemSyncAccessHandle) -> Self {
        Self { pos: 0, handle }
    }
}

impl Read for FileSystemSyncAccessHandleWrapper {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let opts = web_sys::FileSystemReadWriteOptions::new();
        opts.set_at(self.pos as f64);
        let value = self
            .handle
            .read_with_u8_array_and_options(buf, &opts)
            .map_err(|e| std::io::Error::other(format!("Failed to read: {e:?}")))?;
        let bytes_read = value as usize;
        self.pos += bytes_read as u64;
        Ok(bytes_read)
    }
}

impl Seek for FileSystemSyncAccessHandleWrapper {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(offset) => {
                self.pos = offset;
            }
            std::io::SeekFrom::End(offset) => {
                let file_size = self
                    .handle
                    .get_size()
                    .map_err(|e| std::io::Error::other(format!("Failed to get size: {e:?}")))?;
                let new_pos = (file_size as i64) + offset;
                if new_pos < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Seek resulted in a negative file position",
                    ));
                }
                self.pos = new_pos as u64;
            }
            std::io::SeekFrom::Current(offset) => {
                let current_pos = i64::try_from(self.pos).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Position out of range")
                })?;
                let new_pos = current_pos + offset;
                if new_pos < 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Seek resulted in a negative file position",
                    ));
                }
                self.pos = new_pos as u64;
            }
        }
        Ok(self.pos)
    }
}

impl Write for FileSystemSyncAccessHandleWrapper {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let opts = web_sys::FileSystemReadWriteOptions::new();
        opts.set_at(self.pos as f64);
        let value = self
            .handle
            .write_with_u8_array_and_options(buf, &opts)
            .map_err(|e| std::io::Error::other(format!("Failed to write: {e:?}")))?;
        let bytes_written = value as usize;
        self.pos += bytes_written as u64;
        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.handle
            .flush()
            .map_err(|e| std::io::Error::other(format!("Failed to flush: {e:?}")))?;
        Ok(())
    }
}

impl Drop for FileSystemSyncAccessHandleWrapper {
    fn drop(&mut self) {
        self.handle.close();
    }
}

fn is_dicom_file<R: Read + Seek>(reader: &mut R) -> bool {
    let result = reader
        .seek(std::io::SeekFrom::Start(128))
        .and_then(|_| {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            Ok(buf)
        })
        .map(|buf| buf == *b"DICM")
        .unwrap_or(false);

    reader.rewind().ok();
    result
}

#[wasm_bindgen(js_name = "convertViaSyncAccessHandles")]
pub async fn convert_via_sync_access_handles(
    #[wasm_bindgen(js_name = "inputSyncAccessHandles")] input_sync_access_handles: Vec<
        web_sys::FileSystemSyncAccessHandle,
    >,
    #[wasm_bindgen(js_name = "outputSyncAccessHandles")]
    output_sync_access_handle: web_sys::FileSystemSyncAccessHandle,
) -> Result<(), JsValue> {
    crate::panic_hook::set_panic_hook();

    let mut readers = input_sync_access_handles
        .into_iter()
        .map(FileSystemSyncAccessHandleWrapper::from)
        .map(BufReader::new)
        .collect::<Vec<_>>();

    // Remove any files that do not appear to be DICOM files
    readers.retain_mut(is_dicom_file);

    let writer = FileSystemSyncAccessHandleWrapper::from(output_sync_access_handle);

    dicom2tiff::convert_dicom_sources(readers, writer)
        .map_err(|e| JsValue::from(JsError::new(&e.to_string())))?;

    Ok(())
}
