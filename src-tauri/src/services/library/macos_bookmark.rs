use std::path::PathBuf;
#[cfg(test)]
use std::path::Path;

#[cfg(target_os = "macos")]
pub mod native {
    use super::*;
    use core_foundation::base::{kCFAllocatorDefault, TCFType};
    use core_foundation::data::CFData;
    #[cfg(test)]
    use core_foundation::string::CFString;
    use core_foundation::url::{kCFURLPOSIXPathStyle, CFURL};

    /// Resolve without showing any UI (no "locate file" dialog) and without mounting volumes:
    /// an unplugged drive must not trigger a mount attempt during a background sync.
    const RESOLUTION_WITHOUT_UI: usize = 1 << 8;
    const RESOLUTION_WITHOUT_MOUNTING: usize = 1 << 9;

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *const std::ffi::c_void);

        fn CFURLCreateByResolvingBookmarkData(
            allocator: core_foundation::base::CFAllocatorRef,
            bookmark: core_foundation::data::CFDataRef,
            options: usize,
            relative_to_url: core_foundation::url::CFURLRef,
            resource_properties: *const std::ffi::c_void,
            is_stale: *mut u8,
            error: *mut *mut std::ffi::c_void,
        ) -> core_foundation::url::CFURLRef;

        #[cfg(test)]
        fn CFURLCreateBookmarkData(
            allocator: core_foundation::base::CFAllocatorRef,
            url: core_foundation::url::CFURLRef,
            options: usize,
            resource_properties: *const std::ffi::c_void,
            relative_to_url: core_foundation::url::CFURLRef,
            error: *mut *mut std::ffi::c_void,
        ) -> core_foundation::data::CFDataRef;
    }

    /// Resolve an Apple CFURL / NSURL BookmarkData binary blob into a POSIX file path
    pub fn resolve_bookmark(bookmark_bytes: &[u8]) -> Option<PathBuf> {
        if bookmark_bytes.is_empty() {
            return None;
        }

        let data = CFData::from_buffer(bookmark_bytes);
        let mut is_stale: u8 = 0;
        let mut err_ptr: *mut std::ffi::c_void = std::ptr::null_mut();

        unsafe {
            let url_ref = CFURLCreateByResolvingBookmarkData(
                kCFAllocatorDefault,
                data.as_concrete_TypeRef(),
                RESOLUTION_WITHOUT_UI | RESOLUTION_WITHOUT_MOUNTING,
                std::ptr::null(),
                std::ptr::null(),
                &mut is_stale,
                &mut err_ptr,
            );

            // The CFError is returned under the create rule: release it to avoid a leak.
            if !err_ptr.is_null() {
                CFRelease(err_ptr);
            }
            if url_ref.is_null() {
                return None;
            }

            let cf_url = CFURL::wrap_under_create_rule(url_ref);
            let cf_str = cf_url.get_file_system_path(kCFURLPOSIXPathStyle);
            let path_str = cf_str.to_string();

            if path_str.is_empty() {
                None
            } else {
                Some(PathBuf::from(path_str))
            }
        }
    }

    /// Create an Apple CFURL BookmarkData binary blob for a given file path (tests only)
    #[cfg(test)]
    pub fn create_bookmark(path: &Path) -> Option<Vec<u8>> {
        let path_str = path.to_str()?;
        let cf_path = CFString::new(path_str);
        let cf_url = CFURL::from_file_system_path(cf_path, kCFURLPOSIXPathStyle, path.is_dir());

        let mut err_ptr: *mut std::ffi::c_void = std::ptr::null_mut();

        unsafe {
            let data_ref = CFURLCreateBookmarkData(
                kCFAllocatorDefault,
                cf_url.as_concrete_TypeRef(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                &mut err_ptr,
            );

            if !err_ptr.is_null() {
                CFRelease(err_ptr);
            }
            if data_ref.is_null() {
                return None;
            }

            let cf_data = CFData::wrap_under_create_rule(data_ref);
            Some(cf_data.bytes().to_vec())
        }
    }
}

/// Fallback or cross-platform dispatcher
pub fn resolve_bookmark(bookmark_bytes: &[u8]) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        native::resolve_bookmark(bookmark_bytes)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bookmark_bytes;
        None
    }
}

#[cfg(test)]
pub fn create_bookmark(path: &Path) -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        native::create_bookmark(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_bookmark_roundtrip() {
        let path = PathBuf::from("/Applications");
        if path.exists() {
            let bookmark = create_bookmark(&path);
            assert!(bookmark.is_some(), "Should create bookmark for /Applications");
            let resolved = resolve_bookmark(&bookmark.unwrap());
            assert_eq!(resolved, Some(path));
        }
    }
}
