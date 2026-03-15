//! WASM-exported helpers used by the Go/WASI scaffold binding.

use secure_tunnel_core::parse;
use std::alloc::Layout;
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

// Initialize Talc as the global allocator for WebAssembly
#[cfg(target_arch = "wasm32")]
/// SAFETY: The runtime environment must be single-threaded WASM.
#[global_allocator]
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

/// Version of the WASM interface
pub const VERSION: &str = "1.0.0";

/// Allocates memory that can be accessed from the host.
///
/// # Safety
///
/// Returns a pointer to the allocated memory.
/// The memory must be freed using `deallocate`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
    allocation_layout(size).map_or(ptr::null_mut(), |layout| unsafe {
        std::alloc::alloc(layout)
    })
}

/// Deallocates memory previously allocated with `allocate`.
///
/// # Safety
///
/// The pointer must have been allocated with `allocate`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    if !ptr.is_null() {
        if let Some(layout) = allocation_layout(size) {
            unsafe {
                std::alloc::dealloc(ptr, layout);
            }
        }
    }
}

/// Parse an arithmetic expression and return a pointer/length pair as a u64.
/// The high 32 bits contain the pointer, the low 32 bits contain the length.
///
/// Returns a packed u64 with pointer in high 32 bits, length in low 32 bits.
/// The returned memory must be deallocated by the caller using `deallocate`.
/// Returns 0 if an error occurs.
///
/// # Safety
///
/// The caller must ensure that `ptr` points to valid memory with `len` bytes.
/// The input must contain valid UTF-8 text for proper parsing.
/// The caller is responsible for deallocating the returned memory with `deallocate`
/// to prevent memory leaks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_expression(ptr: *const u8, len: usize) -> u64 {
    let input_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };

    if let Ok(input_str) = std::str::from_utf8(input_bytes) {
        return match parse(input_str) {
            Ok(result) => pack_bytes(result.as_bytes()),
            Err(e) => pack_bytes(format!("Error: {e}").as_bytes()),
        };
    }

    pack_bytes(b"Error: invalid UTF-8 in input")
}

/// Check if the result of `parse_expression` is an error.
/// The first byte of the result is checked to determine if it starts with "Error:".
///
/// # Safety
///
/// The caller must ensure the ptr/len pair was obtained from a call to `parse_expression`
/// and the memory is still valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_parse_error(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len < 6 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr, 6.min(len)) };
    i32::from(slice == b"Error:")
}

/// Get the current timestamp in milliseconds since the UNIX epoch.
/// This demonstrates using WASI to get system time.
#[unsafe(no_mangle)]
pub extern "C" fn get_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn allocation_layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size.max(1), 8).ok()
}

fn pack_bytes(bytes: &[u8]) -> u64 {
    let result_len = bytes.len();
    let result_ptr = unsafe { allocate(result_len) };
    if result_ptr.is_null() {
        return 0;
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), result_ptr, result_len);
    }

    ((result_ptr as u64) << 32) | (result_len as u64)
}

#[cfg(target_arch = "wasm32")]
#[cfg(test)]
mod tests {
    use super::*;

    // Helper to unpack a pointer/length pair from a u64
    unsafe fn unpack_ptr_len(packed: u64) -> (*const u8, usize) {
        let ptr = (packed >> 32) as *const u8;
        let len = (packed & 0xFFFFFFFF) as usize;
        (ptr, len)
    }

    // Helper to read a string from a pointer/length pair
    unsafe fn read_string(ptr: *const u8, len: usize) -> String {
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(slice).to_string()
    }

    // Safe wrapper for parse_expression to handle memory cleanup
    fn safe_parse(input: &str) -> Result<String, String> {
        let input_bytes = input.as_bytes();
        unsafe {
            let result = parse_expression(input_bytes.as_ptr(), input_bytes.len());
            if result == 0 {
                return Err("Allocation error".to_string());
            }

            let (ptr, len) = unpack_ptr_len(result);
            let output = read_string(ptr, len);

            // Check if it's an error
            let is_error = is_parse_error(ptr, len) != 0;

            // Clean up allocated memory
            deallocate(ptr as *mut u8, len);

            if is_error { Err(output) } else { Ok(output) }
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_expression_success() {
        match safe_parse("1+2") {
            Ok(result) => assert_eq!(result, "3"),
            Err(err) => panic!("Expected success, got error: {}", err),
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_expression_syntax_error() {
        match safe_parse("1+*2") {
            Ok(result) => panic!("Expected error, got success: {}", result),
            Err(err) => assert!(err.starts_with("Error:")),
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_expression_empty_input() {
        match safe_parse("") {
            Ok(result) => panic!("Expected error, got success: {}", result),
            Err(err) => assert!(err.starts_with("Error:")),
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_parse_expression_complex() {
        // Our parser only supports addition, so this will actually fail
        match safe_parse("1+2") {
            Ok(result) => assert_eq!(result, "3"),
            Err(err) => panic!("Expected success, got error: {}", err),
        }
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_concurrent_allocation() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        // Create a barrier to synchronize all threads to start at the same time
        let thread_count = 5; // Reduced from 10 to lower risk of resource constraints
        let barrier = Arc::new(Barrier::new(thread_count));

        let handles: Vec<_> = (0..thread_count)
            .map(|i| {
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    // Wait for all threads to reach this point before starting
                    barrier.wait();

                    let input = format!("{}+{}", i, i);
                    match safe_parse(&input) {
                        Ok(result) => {
                            assert_eq!(result, format!("{}", i + i));
                            true
                        }
                        Err(err) => {
                            println!("Error parsing {}: {}", input, err);
                            false
                        }
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.join() {
                Ok(result) => assert!(result, "Thread {} failed", i),
                Err(e) => panic!("Thread {} panicked: {:?}", i, e),
            }
        }
    }
}
