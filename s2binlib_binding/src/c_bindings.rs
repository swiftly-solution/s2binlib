/************************************************************************************
 *  S2BinLib - C Bindings with Global Singleton
 *  Copyright (C) 2025  samyyc
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 ***********************************************************************************/

use crate::compat::s2binlib003::{PatternScanCallback, S2BinLib003};
use std::ffi::{c_char, c_void};
use std::sync::Mutex;

// Safety: S2BinLib003 contains a raw pointer to a vtable, but the vtable is static
// and immutable. The vtable functions are designed to be called from any thread.
// The internal S2BinLib state is not shared across threads (each instance is independent).
unsafe impl Send for S2BinLib003 {}

/// Thread-safe global singleton instance of S2BinLib003
static GLOBAL_INSTANCE: Mutex<Option<Box<S2BinLib003>>> = Mutex::new(None);

/// Macro to get the global instance with error handling
macro_rules! with_global_instance {
    ($op:expr) => {{
        let mut guard = match GLOBAL_INSTANCE.lock() {
            Ok(g) => g,
            Err(_) => return -99, // Mutex poisoned error
        };

        match guard.as_mut() {
            Some(instance) => $op(instance.as_mut()),
            None => -1, // Not initialized
        }
    }};
}

/// Macro to get the global instance (const ref) with error handling
macro_rules! with_global_instance_const {
    ($op:expr) => {{
        let guard = match GLOBAL_INSTANCE.lock() {
            Ok(g) => g,
            Err(_) => return -99, // Mutex poisoned error
        };

        match guard.as_ref() {
            Some(instance) => $op(instance.as_ref()),
            None => -1, // Not initialized
        }
    }};
}

/// Macro to generate wrapper functions that take mutable instance with no extra parameters
macro_rules! wrap_method_mut_no_params {
    ($func_name:ident, $method_name:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $func_name() -> i32 {
            with_global_instance!(|instance: &mut S2BinLib003| unsafe {
                let vtable = (*instance.vtable).$method_name;
                vtable(instance)
            })
        }
    };
}

/// Macro to generate wrapper functions that take mutable instance
macro_rules! wrap_method_mut {
    ($func_name:ident, $method_name:ident, $($param:ident: $param_type:ty),*) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $func_name($($param: $param_type),*) -> i32 {
            with_global_instance!(|instance: &mut S2BinLib003| unsafe {
                let vtable = (*instance.vtable).$method_name;
                vtable(instance, $($param),*)
            })
        }
    };
}

/// Macro to generate wrapper functions that take const instance
macro_rules! wrap_method_const {
    ($func_name:ident, $method_name:ident, $($param:ident: $param_type:ty),*) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $func_name($($param: $param_type),*) -> i32 {
            with_global_instance_const!(|instance: &S2BinLib003| unsafe {
                let vtable = (*instance.vtable).$method_name;
                vtable(instance, $($param),*)
            })
        }
    };
}

// ============================================================================
// Lifecycle Functions
// ============================================================================

/// Initialize the global S2BinLib003 instance with auto-detected OS
#[unsafe(no_mangle)]
pub unsafe extern "C" fn s2binlib_initialize(
    game_path: *const c_char,
    game_type: *const c_char,
) -> i32 {
    unsafe {
        let mut guard = match GLOBAL_INSTANCE.lock() {
            Ok(g) => g,
            Err(_) => return -99,
        };

        let mut instance = Box::new(S2BinLib003::new());
        let vtable = (*instance.vtable).initialize;
        let result = vtable(instance.as_mut(), game_path, game_type);

        if result == 0 {
            *guard = Some(instance);
        }

        result
    }
}

/// Initialize the global S2BinLib003 instance with explicit OS
#[unsafe(no_mangle)]
pub unsafe extern "C" fn s2binlib_initialize_with_os(
    game_path: *const c_char,
    game_type: *const c_char,
    os: *const c_char,
) -> i32 {
    unsafe {
        let mut guard = match GLOBAL_INSTANCE.lock() {
            Ok(g) => g,
            Err(_) => return -99,
        };

        let mut instance = Box::new(S2BinLib003::new());
        let vtable = (*instance.vtable).initialize_with_os;
        let result = vtable(instance.as_mut(), game_path, game_type, os);

        if result == 0 {
            *guard = Some(instance);
        }

        result
    }
}

/// Destroy the global S2BinLib003 instance
#[unsafe(no_mangle)]
pub unsafe extern "C" fn s2binlib_destroy() -> i32 {
    unsafe {
        let mut guard = match GLOBAL_INSTANCE.lock() {
            Ok(g) => g,
            Err(_) => return -99,
        };

        if let Some(mut instance) = guard.take() {
            let vtable = (*instance.vtable).destroy;
            vtable(instance.as_mut())
        } else {
            -1 // Not initialized
        }
    }
}

// ============================================================================
// Pattern Scanning Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_pattern_scan,
    pattern_scan,
    binary_name: *const c_char,
    pattern: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_pattern_scan_rva,
    pattern_scan_rva,
    binary_name: *const c_char,
    pattern: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_pattern_scan_all_rva,
    pattern_scan_all_rva,
    binary_name: *const c_char,
    pattern: *const c_char,
    callback: PatternScanCallback,
    user_data: *mut c_void
);

wrap_method_mut!(
    s2binlib_pattern_scan_all,
    pattern_scan_all,
    binary_name: *const c_char,
    pattern: *const c_char,
    callback: PatternScanCallback,
    user_data: *mut c_void
);

// ============================================================================
// VTable Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_find_vtable,
    find_vtable,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vtable_rva,
    find_vtable_rva,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vtable_mangled_rva,
    find_vtable_mangled_rva,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vtable_mangled,
    find_vtable_mangled,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vtable_nested_2_rva,
    find_vtable_nested_2_rva,
    binary_name: *const c_char,
    class1_name: *const c_char,
    class2_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vtable_nested_2,
    find_vtable_nested_2,
    binary_name: *const c_char,
    class1_name: *const c_char,
    class2_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_get_vtable_vfunc_count,
    get_vtable_vfunc_count,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut usize
);

wrap_method_mut!(
    s2binlib_get_vtable_vfunc_count_by_rva,
    get_vtable_vfunc_count_by_rva,
    binary_name: *const c_char,
    vtable_rva: u64,
    result: *mut usize
);

// ============================================================================
// Virtual Function Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_find_vfunc_by_vtbname_rva,
    find_vfunc_by_vtbname_rva,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    vfunc_index: usize,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vfunc_by_vtbname,
    find_vfunc_by_vtbname,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    vfunc_index: usize,
    result: *mut *mut c_void
);

wrap_method_const!(
    s2binlib_find_vfunc_by_vtbptr_rva,
    find_vfunc_by_vtbptr_rva,
    vtable_ptr: *mut c_void,
    vfunc_index: usize,
    result: *mut *mut c_void
);

wrap_method_const!(
    s2binlib_find_vfunc_by_vtbptr,
    find_vfunc_by_vtbptr,
    vtable_ptr: *mut c_void,
    vfunc_index: usize,
    result: *mut *mut c_void
);

// ============================================================================
// Symbol and Export Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_find_symbol,
    find_symbol,
    binary_name: *const c_char,
    symbol_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_symbol_rva,
    find_symbol_rva,
    binary_name: *const c_char,
    symbol_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_export_rva,
    find_export_rva,
    binary_name: *const c_char,
    export_name: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_export,
    find_export,
    binary_name: *const c_char,
    export_name: *const c_char,
    result: *mut *mut c_void
);

// ============================================================================
// String Search Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_find_string_rva,
    find_string_rva,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_string,
    find_string,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void
);

// ============================================================================
// Module Base Address Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_set_module_base_from_pointer,
    set_module_base_from_pointer,
    binary_name: *const c_char,
    pointer: *mut c_void
);

wrap_method_mut!(
    s2binlib_clear_module_base_address,
    clear_module_base_address,
    binary_name: *const c_char
);

wrap_method_const!(
    s2binlib_get_module_base_address,
    get_module_base_address,
    binary_name: *const c_char,
    result: *mut *mut c_void
);

// ============================================================================
// Binary Loading Functions
// ============================================================================

wrap_method_const!(
    s2binlib_is_binary_loaded,
    is_binary_loaded,
    binary_name: *const c_char
);

wrap_method_mut!(
    s2binlib_load_binary,
    load_binary,
    binary_name: *const c_char
);

wrap_method_const!(
    s2binlib_get_binary_path,
    get_binary_path,
    binary_name: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize
);

wrap_method_mut!(
    s2binlib_set_custom_binary_path,
    set_custom_binary_path,
    binary_name: *const c_char,
    path: *const c_char,
    os: *const c_char
);

wrap_method_mut!(
    s2binlib_unload_binary,
    unload_binary,
    binary_name: *const c_char
);

wrap_method_mut_no_params!(s2binlib_unload_all_binaries, unload_all_binaries);

// ============================================================================
// Memory Read Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_read_by_file_offset,
    read_by_file_offset,
    binary_name: *const c_char,
    file_offset: u64,
    buffer: *mut u8,
    buffer_size: usize
);

wrap_method_mut!(
    s2binlib_read_by_rva,
    read_by_rva,
    binary_name: *const c_char,
    rva: u64,
    buffer: *mut u8,
    buffer_size: usize
);

wrap_method_mut!(
    s2binlib_read_by_mem_address,
    read_by_mem_address,
    binary_name: *const c_char,
    mem_address: u64,
    buffer: *mut u8,
    buffer_size: usize
);

// ============================================================================
// Object Pointer Functions
// ============================================================================

wrap_method_const!(
    s2binlib_get_object_ptr_vtable_name,
    get_object_ptr_vtable_name,
    object_ptr: *const c_void,
    buffer: *mut c_char,
    buffer_size: usize
);

wrap_method_const!(
    s2binlib_object_ptr_has_vtable,
    object_ptr_has_vtable,
    object_ptr: *const c_void
);

wrap_method_const!(
    s2binlib_object_ptr_has_base_class,
    object_ptr_has_base_class,
    object_ptr: *const c_void,
    base_class_name: *const c_char
);

// ============================================================================
// Cross-Reference Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_dump_xrefs,
    dump_xrefs,
    binary_name: *const c_char
);

wrap_method_const!(
    s2binlib_get_xrefs_count,
    get_xrefs_count,
    binary_name: *const c_char,
    target_rva: *mut c_void
);

wrap_method_const!(
    s2binlib_get_xrefs_cached,
    get_xrefs_cached,
    binary_name: *const c_char,
    target_rva: *mut c_void,
    buffer: *mut *mut c_void,
    buffer_size: usize
);

wrap_method_const!(
    s2binlib_follow_xref_mem_to_mem,
    follow_xref_mem_to_mem,
    mem_address: *const c_void,
    target_address_out: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_follow_xref_rva_to_mem,
    follow_xref_rva_to_mem,
    binary_name: *const c_char,
    rva: u64,
    target_address_out: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_follow_xref_rva_to_rva,
    follow_xref_rva_to_rva,
    binary_name: *const c_char,
    rva: u64,
    target_rva_out: *mut u64
);

// ============================================================================
// Function Discovery Helpers
// ============================================================================

wrap_method_mut!(
    s2binlib_dump_vtables,
    dump_vtables,
    binary_name: *const c_char
);

wrap_method_mut!(
    s2binlib_find_func_start_rva,
    find_func_start_rva,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut u64
);

wrap_method_mut!(
    s2binlib_find_func_start,
    find_func_start,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_vfunc_start_rva,
    find_vfunc_start_rva,
    binary_name: *const c_char,
    include_rva: u64,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    vfunc_rva_out: *mut u64
);

wrap_method_mut!(
    s2binlib_find_vfunc_start,
    find_vfunc_start,
    binary_name: *const c_char,
    include_rva: u64,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_xref_func_start_rva,
    find_xref_func_start_rva,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut u64
);

wrap_method_mut!(
    s2binlib_find_xref_func_start,
    find_xref_func_start,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_xref_func_with_string,
    find_xref_func_with_string,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_xref_func_with_string_rva,
    find_xref_func_with_string_rva,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut u64
);

wrap_method_mut!(
    s2binlib_find_vfunc_with_string_rva,
    find_vfunc_with_string_rva,
    binary_name: *const c_char,
    string: *const c_char,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    vfunc_rva_out: *mut u64
);

wrap_method_mut!(
    s2binlib_find_vfunc_with_string,
    find_vfunc_with_string,
    binary_name: *const c_char,
    string: *const c_char,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    result: *mut *mut c_void
);

wrap_method_mut!(
    s2binlib_find_func_with_string_rva,
    find_func_with_string_rva,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut u64
);

wrap_method_mut!(
    s2binlib_find_func_with_string,
    find_func_with_string,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void
);

// ============================================================================
// JIT and Trampoline Functions
// ============================================================================

wrap_method_mut!(
    s2binlib_install_trampoline,
    install_trampoline,
    mem_address: *mut c_void,
    trampoline_address_out: *mut *mut c_void
);

// ============================================================================
// NetworkVar Functions
// ============================================================================

wrap_method_const!(
    s2binlib_find_networkvar_vtable_statechanged_rva,
    find_networkvar_vtable_statechanged_rva,
    vtable_rva: u64,
    result: *mut u64
);

wrap_method_const!(
    s2binlib_find_networkvar_vtable_statechanged,
    find_networkvar_vtable_statechanged,
    vtable_mem_address: u64,
    result: *mut u64
);

wrap_method_mut!(
    s2binlib_make_sig_rva,
    make_sig_rva,
    binary_name: *const c_char,
    func_rva: u64,
    result_out: *mut c_char,
    result_out_size: usize
);