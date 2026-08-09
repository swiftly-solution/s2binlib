/************************************************************************************
 *  S2BinLib - A static library that helps resolving memory from binary file
 *  and map to absolute memory address, targeting source 2 game engine.
 *  Copyright (C) 2025-2026  samyyc
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

use s2binlib::S2BinLib;
use std::ffi::{CStr, c_char, c_void};

pub type PatternScanCallback =
    unsafe extern "C" fn(index: usize, address: *mut c_void, user_data: *mut c_void);

#[cfg(feature = "debug_c_bindings")]
macro_rules! c_debug {
    ($($arg:tt)*) => {
        println!("[S2BinLib004 Debug] {}", format!($($arg)*));
    };
}

#[cfg(not(feature = "debug_c_bindings"))]
macro_rules! c_debug {
    ($($arg:tt)*) => {};
}

macro_rules! return_error {
    ($code:expr, $msg:expr) => {{
        c_debug!("Error {}: {} (at {}:{})", $code, $msg, file!(), line!());
        return $code;
    }};
    ($code:expr) => {{
        c_debug!("Error {} (at {}:{})", $code, file!(), line!());
        return $code;
    }};
}

macro_rules! get_s2binlib_mut {
    ($this:expr) => {{
        let s2binlib = unsafe { (*$this).s2binlib.as_mut() };
        if s2binlib.is_none() {
            return_error!(-1, "S2BinLib004 is not initialized");
        }
        s2binlib.unwrap()
    }};
}

macro_rules! get_s2binlib {
    ($this:expr) => {{
        let s2binlib = unsafe { (*$this).s2binlib.as_ref() };
        if s2binlib.is_none() {
            return_error!(-1, "S2BinLib004 is not initialized");
        }
        s2binlib.unwrap()
    }};
}

macro_rules! cstr_to_str {
    ($cstr:expr) => {{
        unsafe {
            match CStr::from_ptr($cstr).to_str() {
                Ok(s) => s,
                Err(_) => return_error!(-2, "Failed to convert C string to UTF-8"),
            }
        }
    }};
}

macro_rules! ensure_binary_loaded {
    ($s2binlib:expr, $binary_name:expr) => {{
        if !$s2binlib.is_binary_loaded($binary_name) {
            $s2binlib.load_binary($binary_name);
        }
    }};
}

macro_rules! check_null {
    ($($ptr:expr),+) => {{
        $(
            if $ptr.is_null() {
                return_error!(-2, "invalid parameter: null pointer");
            }
        )+
    }};
}

#[repr(C)]
pub struct S2BinLib004 {
    pub vtable: *const S2BinLib004VTable,
    s2binlib: Option<S2BinLib<'static>>,
}

impl S2BinLib004 {
    pub fn new() -> Self {
        Self {
            vtable: &VTABLE,
            s2binlib: None,
        }
    }

    pub(crate) fn dump_strings_to_json(&mut self, binary_name: &str, output_path: &str) -> i32 {
        let Some(s2binlib) = self.s2binlib.as_mut() else {
            c_debug!("Error -1: S2BinLib004 is not initialized");
            return -1;
        };

        if !s2binlib.is_binary_loaded(binary_name) {
            s2binlib.load_binary(binary_name);
        }

        match s2binlib.dump_strings_to_json(binary_name, output_path) {
            Ok(()) => 0,
            Err(error) => {
                c_debug!("Error -4: Failed to dump strings to JSON: {}", error);
                -4
            }
        }
    }
}

#[repr(C)]
pub struct S2BinLib004VTable {
    pub initialize: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        game_path: *const c_char,
        game_type: *const c_char,
    ) -> i32,
    pub initialize_with_os: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        game_path: *const c_char,
        game_type: *const c_char,
        os: *const c_char,
    ) -> i32,
    pub pattern_scan: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        pattern: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vtable: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_symbol: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        symbol_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub set_module_base_from_pointer: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        pointer: *mut c_void,
    ) -> i32,
    pub clear_module_base_address:
        unsafe extern "C" fn(this: *mut S2BinLib004, binary_name: *const c_char) -> i32,
    pub set_custom_binary_path: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        path: *const c_char,
        os: *const c_char,
    ) -> i32,
    pub get_module_base_address: unsafe extern "C" fn(
        this: *const S2BinLib004,
        binary_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub is_binary_loaded:
        unsafe extern "C" fn(this: *const S2BinLib004, binary_name: *const c_char) -> i32,
    pub load_binary:
        unsafe extern "C" fn(this: *mut S2BinLib004, binary_name: *const c_char) -> i32,
    pub get_binary_path: unsafe extern "C" fn(
        this: *const S2BinLib004,
        binary_name: *const c_char,
        buffer: *mut c_char,
        buffer_size: usize,
    ) -> i32,
    pub find_vtable_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vtable_mangled_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vtable_mangled: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vtable_nested_2_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        class1_name: *const c_char,
        class2_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vtable_nested_2: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        class1_name: *const c_char,
        class2_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub get_vtable_vfunc_count: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        result: *mut usize,
    ) -> i32,
    pub get_vtable_vfunc_count_by_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_rva: u64,
        result: *mut usize,
    ) -> i32,
    pub pattern_scan_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        pattern: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub pattern_scan_all_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        pattern: *const c_char,
        callback: PatternScanCallback,
        user_data: *mut c_void,
    ) -> i32,
    pub pattern_scan_all: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        pattern: *const c_char,
        callback: PatternScanCallback,
        user_data: *mut c_void,
    ) -> i32,
    pub find_export_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        export_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_export: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        export_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_symbol_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        symbol_name: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub read_by_file_offset: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        file_offset: u64,
        buffer: *mut u8,
        buffer_size: usize,
    ) -> i32,
    pub read_by_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        rva: u64,
        buffer: *mut u8,
        buffer_size: usize,
    ) -> i32,
    pub read_by_mem_address: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        mem_address: u64,
        buffer: *mut u8,
        buffer_size: usize,
    ) -> i32,
    pub find_vfunc_by_vtbname_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        vfunc_index: usize,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vfunc_by_vtbname: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        vtable_name: *const c_char,
        vfunc_index: usize,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vfunc_by_vtbptr_rva: unsafe extern "C" fn(
        this: *const S2BinLib004,
        vtable_ptr: *mut c_void,
        vfunc_index: usize,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vfunc_by_vtbptr: unsafe extern "C" fn(
        this: *const S2BinLib004,
        vtable_ptr: *mut c_void,
        vfunc_index: usize,
        result: *mut *mut c_void,
    ) -> i32,
    pub get_object_ptr_vtable_name: unsafe extern "C" fn(
        this: *const S2BinLib004,
        object_ptr: *const c_void,
        buffer: *mut c_char,
        buffer_size: usize,
    ) -> i32,
    pub object_ptr_has_vtable:
        unsafe extern "C" fn(this: *const S2BinLib004, object_ptr: *const c_void) -> i32,
    pub object_ptr_has_base_class: unsafe extern "C" fn(
        this: *const S2BinLib004,
        object_ptr: *const c_void,
        base_class_name: *const c_char,
    ) -> i32,
    pub find_string_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_string: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub dump_xrefs: unsafe extern "C" fn(this: *mut S2BinLib004, binary_name: *const c_char) -> i32,
    pub get_xrefs_count: unsafe extern "C" fn(
        this: *const S2BinLib004,
        binary_name: *const c_char,
        target_rva: *mut c_void,
    ) -> i32,
    pub get_xrefs_cached: unsafe extern "C" fn(
        this: *const S2BinLib004,
        binary_name: *const c_char,
        target_rva: *mut c_void,
        buffer: *mut *mut c_void,
        buffer_size: usize,
    ) -> i32,
    pub unload_binary:
        unsafe extern "C" fn(this: *mut S2BinLib004, binary_name: *const c_char) -> i32,
    pub unload_all_binaries: unsafe extern "C" fn(this: *mut S2BinLib004) -> i32,
    pub install_trampoline: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        mem_address: *mut c_void,
        trampoline_address_out: *mut *mut c_void,
    ) -> i32,
    pub follow_xref_mem_to_mem: unsafe extern "C" fn(
        this: *const S2BinLib004,
        mem_address: *const c_void,
        target_address_out: *mut *mut c_void,
    ) -> i32,
    pub follow_xref_rva_to_mem: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        rva: u64,
        target_address_out: *mut *mut c_void,
    ) -> i32,
    pub follow_xref_rva_to_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        rva: u64,
        target_rva_out: *mut u64,
    ) -> i32,
    pub dump_vtables:
        unsafe extern "C" fn(this: *mut S2BinLib004, binary_name: *const c_char) -> i32,
    pub find_func_start_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        include_rva: u64,
        result: *mut u64,
    ) -> i32,
    pub find_func_start: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        include_rva: u64,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_vfunc_start_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        include_rva: u64,
        vtable_name_out: *mut c_char,
        vtable_name_out_size: usize,
        vfunc_index_out: *mut usize,
        vfunc_rva_out: *mut u64,
    ) -> i32,
    pub find_vfunc_start: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        include_rva: u64,
        vtable_name_out: *mut c_char,
        vtable_name_out_size: usize,
        vfunc_index_out: *mut usize,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_xref_func_start_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        include_rva: u64,
        result: *mut u64,
    ) -> i32,
    pub find_xref_func_start: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        include_rva: u64,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_xref_func_with_string: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_xref_func_with_string_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        result: *mut u64,
    ) -> i32,
    pub find_vfunc_with_string_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        vtable_name_out: *mut c_char,
        vtable_name_out_size: usize,
        vfunc_index_out: *mut usize,
        vfunc_rva_out: *mut u64,
    ) -> i32,
    pub find_vfunc_with_string: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        vtable_name_out: *mut c_char,
        vtable_name_out_size: usize,
        vfunc_index_out: *mut usize,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_func_with_string_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        result: *mut u64,
    ) -> i32,
    pub find_func_with_string: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        string: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_networkvar_vtable_statechanged_rva:
        unsafe extern "C" fn(this: *const S2BinLib004, vtable_rva: u64, result: *mut u64) -> i32,
    pub find_networkvar_vtable_statechanged: unsafe extern "C" fn(
        this: *const S2BinLib004,
        vtable_mem_address: u64,
        result: *mut u64,
    ) -> i32,
    pub make_sig_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        func_rva: u64,
        result_out: *mut c_char,
        result_out_size: usize,
    ) -> i32,
    pub destroy: unsafe extern "C" fn(this: *mut S2BinLib004) -> i32,
    pub find_call_with_xref_arg_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        func_start_rva: u64,
        target_rva: u64,
        result: *mut u64,
    ) -> i32,
    pub find_call_with_xref_arg: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        func_start: *mut c_void,
        target: *mut c_void,
        result: *mut *mut c_void,
    ) -> i32,
    pub find_call_with_string_arg_rva: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        func_start_rva: u64,
        string: *const c_char,
        result: *mut u64,
    ) -> i32,
    pub find_call_with_string_arg: unsafe extern "C" fn(
        this: *mut S2BinLib004,
        binary_name: *const c_char,
        func_start: *mut c_void,
        string: *const c_char,
        result: *mut *mut c_void,
    ) -> i32,
}

static VTABLE: S2BinLib004VTable = S2BinLib004VTable {
    initialize: Initialize,
    initialize_with_os: InitializeWithOs,
    pattern_scan: PatternScan,
    find_vtable: FindVtable,
    find_symbol: FindSymbol,
    set_module_base_from_pointer: SetModuleBaseFromPointer,
    clear_module_base_address: ClearModuleBaseAddress,
    set_custom_binary_path: SetCustomBinaryPath,
    get_module_base_address: GetModuleBaseAddress,
    is_binary_loaded: IsBinaryLoaded,
    load_binary: LoadBinary,
    get_binary_path: GetBinaryPath,
    find_vtable_rva: FindVtableRva,
    find_vtable_mangled_rva: FindVtableMangledRva,
    find_vtable_mangled: FindVtableMangled,
    find_vtable_nested_2_rva: FindVtableNested2Rva,
    find_vtable_nested_2: FindVtableNested2,
    get_vtable_vfunc_count: GetVtableVfuncCount,
    get_vtable_vfunc_count_by_rva: GetVtableVfuncCountByRva,
    pattern_scan_rva: PatternScanRva,
    pattern_scan_all_rva: PatternScanAllRva,
    pattern_scan_all: PatternScanAll,
    find_export_rva: FindExportRva,
    find_export: FindExport,
    find_symbol_rva: FindSymbolRva,
    read_by_file_offset: ReadByFileOffset,
    read_by_rva: ReadByRva,
    read_by_mem_address: ReadByMemAddress,
    find_vfunc_by_vtbname_rva: FindVfuncByVtbnameRva,
    find_vfunc_by_vtbname: FindVfuncByVtbname,
    find_vfunc_by_vtbptr_rva: FindVfuncByVtbptrRva,
    find_vfunc_by_vtbptr: FindVfuncByVtbptr,
    get_object_ptr_vtable_name: GetObjectPtrVtableName,
    object_ptr_has_vtable: ObjectPtrHasVtable,
    object_ptr_has_base_class: ObjectPtrHasBaseClass,
    find_string_rva: FindStringRva,
    find_string: FindString,
    dump_xrefs: DumpXrefs,
    get_xrefs_count: GetXrefsCount,
    get_xrefs_cached: GetXrefsCached,
    unload_binary: UnloadBinary,
    unload_all_binaries: UnloadAllBinaries,
    install_trampoline: InstallTrampoline,
    follow_xref_mem_to_mem: FollowXrefMemToMem,
    follow_xref_rva_to_mem: FollowXrefRvaToMem,
    follow_xref_rva_to_rva: FollowXrefRvaToRva,
    dump_vtables: DumpVtables,
    find_func_start_rva: FindFuncStartRva,
    find_func_start: FindFuncStart,
    find_vfunc_start_rva: FindVfuncStartRva,
    find_vfunc_start: FindVfuncStart,
    find_xref_func_start_rva: FindXrefFuncStartRva,
    find_xref_func_start: FindXrefFuncStart,
    find_xref_func_with_string: FindXrefFuncWithString,
    find_xref_func_with_string_rva: FindXrefFuncWithStringRva,
    find_vfunc_with_string_rva: FindVfuncWithStringRva,
    find_vfunc_with_string: FindVfuncWithString,
    find_func_with_string_rva: FindFuncWithStringRva,
    find_func_with_string: FindFuncWithString,
    find_networkvar_vtable_statechanged_rva: FindNetworkvarVtableStatechangedRva,
    find_networkvar_vtable_statechanged: FindNetworkvarVtableStatechanged,
    make_sig_rva: MakeSigRva,
    destroy: Destroy,
    find_call_with_xref_arg_rva: FindCallWithXrefArgRva,
    find_call_with_xref_arg: FindCallWithXrefArg,
    find_call_with_string_arg_rva: FindCallWithStringArgRva,
    find_call_with_string_arg: FindCallWithStringArg,
};

unsafe extern "C" fn Initialize(
    this: *mut S2BinLib004,
    game_path: *const c_char,
    game_type: *const c_char,
) -> i32 {
    check_null!(this, game_path, game_type);

    let game_path_str = cstr_to_str!(game_path);
    let game_type_str = cstr_to_str!(game_type);

    let os_str = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return_error!(-2, "Unsupported operating system");
    };

    (*this).s2binlib = Some(S2BinLib::new(game_path_str, game_type_str, os_str));
    0
}

unsafe extern "C" fn InitializeWithOs(
    this: *mut S2BinLib004,
    game_path: *const c_char,
    game_type: *const c_char,
    os: *const c_char,
) -> i32 {
    check_null!(this, game_path, game_type, os);

    let game_path_str = cstr_to_str!(game_path);
    let game_type_str = cstr_to_str!(game_type);
    let os_str = cstr_to_str!(os);

    (*this).s2binlib = Some(S2BinLib::new(game_path_str, game_type_str, os_str));
    0
}

unsafe extern "C" fn PatternScan(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    pattern: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, pattern, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let pattern_str = cstr_to_str!(pattern);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.pattern_scan(binary_name_str, pattern_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVtable(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vtable(binary_name_str, vtable_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindSymbol(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    symbol_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, symbol_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let symbol_name_str = cstr_to_str!(symbol_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_symbol(binary_name_str, symbol_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn SetModuleBaseFromPointer(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    pointer: *mut c_void,
) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    s2binlib.set_module_base_from_pointer(binary_name_str, pointer as u64);
    0
}

unsafe extern "C" fn ClearModuleBaseAddress(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    s2binlib.clear_module_base_address(binary_name_str);
    0
}

unsafe extern "C" fn SetCustomBinaryPath(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    path: *const c_char,
    os: *const c_char,
) -> i32 {
    check_null!(this, binary_name, path, os);

    let binary_name_str = cstr_to_str!(binary_name);
    let path_str = cstr_to_str!(path);
    let os_str = cstr_to_str!(os);
    let s2binlib = get_s2binlib_mut!(this);

    match s2binlib.set_custom_binary_path(binary_name_str, path_str, os_str) {
        Ok(_) => 0,
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn GetModuleBaseAddress(
    this: *const S2BinLib004,
    binary_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib!(this);

    match s2binlib.get_module_base_address(binary_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn IsBinaryLoaded(this: *const S2BinLib004, binary_name: *const c_char) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib!(this);

    if s2binlib.is_binary_loaded(binary_name_str) {
        1
    } else {
        0
    }
}

unsafe extern "C" fn LoadBinary(this: *mut S2BinLib004, binary_name: *const c_char) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    s2binlib.load_binary(binary_name_str);
    0
}

unsafe extern "C" fn GetBinaryPath(
    this: *const S2BinLib004,
    binary_name: *const c_char,
    buffer: *mut c_char,
    buffer_size: usize,
) -> i32 {
    if this.is_null() || binary_name.is_null() || buffer.is_null() || buffer_size == 0 {
        return -2;
    }

    let binary_name_str = match CStr::from_ptr(binary_name).to_str() {
        Ok(s) => s,
        Err(_) => return_error!(-2, "Failed to convert C string to UTF-8"),
    };

    let s2binlib = get_s2binlib!(this);

    let path = s2binlib.get_binary_path(binary_name_str);
    let path_bytes = path.as_bytes();

    if path_bytes.len() + 1 > buffer_size {
        return -3;
    }

    std::ptr::copy_nonoverlapping(path_bytes.as_ptr(), buffer as *mut u8, path_bytes.len());
    *(buffer.add(path_bytes.len())) = 0;

    0
}

unsafe extern "C" fn FindVtableRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vtable_rva(binary_name_str, vtable_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVtableMangledRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vtable_mangled_rva(binary_name_str, vtable_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVtableMangled(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vtable_mangled(binary_name_str, vtable_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVtableNested2Rva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    class1_name: *const c_char,
    class2_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, class1_name, class2_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let class1_name_str = cstr_to_str!(class1_name);
    let class2_name_str = cstr_to_str!(class2_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vtable_nested_2_rva(binary_name_str, class1_name_str, class2_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVtableNested2(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    class1_name: *const c_char,
    class2_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, class1_name, class2_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let class1_name_str = cstr_to_str!(class1_name);
    let class2_name_str = cstr_to_str!(class2_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vtable_nested_2(binary_name_str, class1_name_str, class2_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn GetVtableVfuncCount(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    result: *mut usize,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.get_vtable_vfunc_count(binary_name_str, vtable_name_str) {
        Ok(count) => {
            *result = count;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn GetVtableVfuncCountByRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_rva: u64,
    result: *mut usize,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.get_vtable_vfunc_count_by_rva(binary_name_str, vtable_rva) {
        Ok(count) => {
            *result = count;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn PatternScanRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    pattern: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, pattern, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let pattern_str = cstr_to_str!(pattern);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.pattern_scan_rva(binary_name_str, pattern_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn PatternScanAllRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    pattern: *const c_char,
    callback: PatternScanCallback,
    user_data: *mut c_void,
) -> i32 {
    check_null!(this, binary_name, pattern);

    let binary_name_str = cstr_to_str!(binary_name);
    let pattern_str = cstr_to_str!(pattern);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.pattern_scan_all_rva(binary_name_str, pattern_str, |index, addr| {
        callback(index, addr as *mut c_void, user_data);
        true
    }) {
        Ok(_) => 0,
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn PatternScanAll(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    pattern: *const c_char,
    callback: PatternScanCallback,
    user_data: *mut c_void,
) -> i32 {
    check_null!(this, binary_name, pattern);

    let binary_name_str = cstr_to_str!(binary_name);
    let pattern_str = cstr_to_str!(pattern);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.pattern_scan_all(binary_name_str, pattern_str, |index, addr| {
        callback(index, addr as *mut c_void, user_data);
        true
    }) {
        Ok(_) => 0,
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindExportRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    export_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, export_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let export_name_str = cstr_to_str!(export_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_export_rva(binary_name_str, export_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindExport(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    export_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, export_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let export_name_str = cstr_to_str!(export_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_export(binary_name_str, export_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindSymbolRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    symbol_name: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, symbol_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let symbol_name_str = cstr_to_str!(symbol_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_symbol_rva(binary_name_str, symbol_name_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn ReadByFileOffset(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    file_offset: u64,
    buffer: *mut u8,
    buffer_size: usize,
) -> i32 {
    check_null!(this, binary_name, buffer);
    if buffer_size == 0 {
        return -2;
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.read_by_file_offset(binary_name_str, file_offset, buffer_size) {
        Ok(bytes) => {
            let copy_size = bytes.len().min(buffer_size);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, copy_size);
            }
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn ReadByRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    rva: u64,
    buffer: *mut u8,
    buffer_size: usize,
) -> i32 {
    check_null!(this, binary_name, buffer);
    if buffer_size == 0 {
        return -2;
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.read_by_rva(binary_name_str, rva, buffer_size) {
        Ok(bytes) => {
            let copy_size = bytes.len().min(buffer_size);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, copy_size);
            }
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn ReadByMemAddress(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    mem_address: u64,
    buffer: *mut u8,
    buffer_size: usize,
) -> i32 {
    check_null!(this, binary_name, buffer);
    if buffer_size == 0 {
        return -2;
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.read_by_mem_address(binary_name_str, mem_address, buffer_size) {
        Ok(bytes) => {
            let copy_size = bytes.len().min(buffer_size);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, copy_size);
            }
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncByVtbnameRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    vfunc_index: usize,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vfunc_by_vtbname_rva(binary_name_str, vtable_name_str, vfunc_index) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncByVtbname(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    vtable_name: *const c_char,
    vfunc_index: usize,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, vtable_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let vtable_name_str = cstr_to_str!(vtable_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vfunc_by_vtbname(binary_name_str, vtable_name_str, vfunc_index) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncByVtbptrRva(
    this: *const S2BinLib004,
    vtable_ptr: *mut c_void,
    vfunc_index: usize,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, result);

    let s2binlib = get_s2binlib!(this);

    match s2binlib.find_vfunc_by_vtbptr_rva(vtable_ptr as u64, vfunc_index) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncByVtbptr(
    this: *const S2BinLib004,
    vtable_ptr: *mut c_void,
    vfunc_index: usize,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, result);

    let s2binlib = get_s2binlib!(this);

    match s2binlib.find_vfunc_by_vtbptr(vtable_ptr as u64, vfunc_index) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn GetObjectPtrVtableName(
    this: *const S2BinLib004,
    object_ptr: *const c_void,
    buffer: *mut c_char,
    buffer_size: usize,
) -> i32 {
    check_null!(this, object_ptr, buffer);
    if buffer_size == 0 {
        return_error!(-2, "invalid parameter: buffer_size is zero");
    }

    let s2binlib = get_s2binlib!(this);

    match s2binlib.get_object_ptr_vtable_name(object_ptr as u64) {
        Ok(name) => {
            let name_bytes = name.as_bytes();
            if name_bytes.len() + 1 > buffer_size {
                return_error!(-3, "buffer too small to store vtable name");
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    name_bytes.as_ptr(),
                    buffer as *mut u8,
                    name_bytes.len(),
                );
                *buffer.add(name_bytes.len()) = 0;
            }

            0
        }
        Err(_err) => return_error!(-4, "Failed to get vtable info"),
    }
}

unsafe extern "C" fn ObjectPtrHasVtable(
    this: *const S2BinLib004,
    object_ptr: *const c_void,
) -> i32 {
    check_null!(this, object_ptr);

    let s2binlib = get_s2binlib!(this);

    if s2binlib.object_ptr_has_vtable(object_ptr as u64) {
        1
    } else {
        0
    }
}

unsafe extern "C" fn ObjectPtrHasBaseClass(
    this: *const S2BinLib004,
    object_ptr: *const c_void,
    base_class_name: *const c_char,
) -> i32 {
    check_null!(this, object_ptr, base_class_name);

    let base_class_name_str = cstr_to_str!(base_class_name);
    let s2binlib = get_s2binlib!(this);

    match s2binlib.object_ptr_has_base_class(object_ptr as u64, base_class_name_str) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => return_error!(-4, "Failed to get vtable info"),
    }
}

unsafe extern "C" fn FindStringRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_string_rva(binary_name_str, string_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindString(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_string(binary_name_str, string_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn DumpXrefs(this: *mut S2BinLib004, binary_name: *const c_char) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.dump_xrefs(binary_name_str) {
        Ok(_) => 0,
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn GetXrefsCount(
    this: *const S2BinLib004,
    binary_name: *const c_char,
    target_rva: *mut c_void,
) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib!(this);

    match s2binlib.find_xrefs_cached(binary_name_str, target_rva as u64) {
        Some(xrefs) => xrefs.len() as i32,
        None => -3,
    }
}

unsafe extern "C" fn GetXrefsCached(
    this: *const S2BinLib004,
    binary_name: *const c_char,
    target_rva: *mut c_void,
    buffer: *mut *mut c_void,
    buffer_size: usize,
) -> i32 {
    check_null!(this, binary_name, buffer);
    if buffer_size == 0 {
        return -2;
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib!(this);

    match s2binlib.find_xrefs_cached(binary_name_str, target_rva as u64) {
        Some(xrefs) => {
            if xrefs.len() * std::mem::size_of::<*mut c_void>() > buffer_size {
                return -4;
            }

            let copy_count = xrefs.len();
            unsafe {
                for (i, addr) in xrefs.iter().enumerate() {
                    *buffer.add(i) = *addr as *mut c_void;
                }
            }
            copy_count as i32
        }
        None => -3,
    }
}

unsafe extern "C" fn UnloadBinary(this: *mut S2BinLib004, binary_name: *const c_char) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    s2binlib.unload_binary(binary_name_str);
    0
}

unsafe extern "C" fn UnloadAllBinaries(this: *mut S2BinLib004) -> i32 {
    check_null!(this);

    let s2binlib = get_s2binlib_mut!(this);

    s2binlib.unload_all_binaries();
    0
}

unsafe extern "C" fn InstallTrampoline(
    this: *mut S2BinLib004,
    mem_address: *mut c_void,
    trampoline_address_out: *mut *mut c_void,
) -> i32 {
    check_null!(this);

    let s2binlib = get_s2binlib_mut!(this);

    match s2binlib.install_trampoline(mem_address as u64) {
        Ok(address) => {
            *trampoline_address_out = address as *mut c_void;
            0
        }
        Err(_) => return_error!(-3, "Failed to install trampoline"),
    }
}

unsafe extern "C" fn FollowXrefMemToMem(
    this: *const S2BinLib004,
    mem_address: *const c_void,
    target_address_out: *mut *mut c_void,
) -> i32 {
    check_null!(this, mem_address, target_address_out);

    let s2binlib = get_s2binlib!(this);

    match s2binlib.follow_xref_mem_to_mem(mem_address as u64) {
        Ok(target) => {
            *target_address_out = target as *mut c_void;
            0
        }
        Err(_) => return_error!(-3, "No valid xref found or invalid instruction"),
    }
}

unsafe extern "C" fn FollowXrefRvaToMem(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    rva: u64,
    target_address_out: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, target_address_out);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.follow_xref_rva_to_mem(binary_name_str, rva) {
        Ok(target) => {
            *target_address_out = target as *mut c_void;
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn FollowXrefRvaToRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    rva: u64,
    target_rva_out: *mut u64,
) -> i32 {
    check_null!(this, binary_name, target_rva_out);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.follow_xref_rva_to_rva(binary_name_str, rva) {
        Ok(target) => {
            *target_rva_out = target;
            0
        }
        Err(_) => return_error!(-3, "Failed to load binary or operation failed"),
    }
}

unsafe extern "C" fn DumpVtables(this: *mut S2BinLib004, binary_name: *const c_char) -> i32 {
    check_null!(this, binary_name);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.dump_vtables(binary_name_str) {
        Ok(_) => 0,
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindFuncStartRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut u64,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_func_start_rva(binary_name_str, include_rva) {
        Ok(addr) => {
            *result = addr;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindFuncStart(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_func_start(binary_name_str, include_rva) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncStartRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    include_rva: u64,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    vfunc_rva_out: *mut u64,
) -> i32 {
    check_null!(
        this,
        binary_name,
        vtable_name_out,
        vfunc_index_out,
        vfunc_rva_out
    );
    if vtable_name_out_size == 0 {
        return_error!(-2, "invalid parameter: buffer_size is zero");
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vfunc_start_rva(binary_name_str, include_rva) {
        Some((vtable_info, index, rva)) => {
            let name_bytes = vtable_info.type_name.as_bytes();
            if name_bytes.len() + 1 > vtable_name_out_size {
                return_error!(-3, "buffer too small to store vtable name");
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    name_bytes.as_ptr(),
                    vtable_name_out as *mut u8,
                    name_bytes.len(),
                );
                *vtable_name_out.add(name_bytes.len()) = 0;
            }

            *vfunc_index_out = index;
            *vfunc_rva_out = rva;
            0
        }
        None => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncStart(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    include_rva: u64,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(
        this,
        binary_name,
        vtable_name_out,
        vfunc_index_out,
        result
    );
    if vtable_name_out_size == 0 {
        return_error!(-2, "invalid parameter: buffer_size is zero");
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vfunc_start_rva(binary_name_str, include_rva) {
        Some((vtable_info, index, rva)) => {
            let name_bytes = vtable_info.type_name.as_bytes();
            if name_bytes.len() + 1 > vtable_name_out_size {
                return_error!(-3, "buffer too small to store vtable name");
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    name_bytes.as_ptr(),
                    vtable_name_out as *mut u8,
                    name_bytes.len(),
                );
                *vtable_name_out.add(name_bytes.len()) = 0;
            }

            *vfunc_index_out = index;
            match s2binlib.rva_to_mem_address(binary_name_str, rva) {
                Ok(addr) => {
                    *result = addr as *mut c_void;
                    0
                }
                Err(_) => return_error!(-4, "Pattern not found or operation failed"),
            }
        }
        None => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindXrefFuncStartRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut u64,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_xref_func_start_rva(binary_name_str, include_rva) {
        Ok(addr) => {
            *result = addr;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindXrefFuncStart(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    include_rva: u64,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_xref_func_start_rva(binary_name_str, include_rva) {
        Ok(addr) => match s2binlib.rva_to_mem_address(binary_name_str, addr) {
            Ok(mem) => {
                *result = mem as *mut c_void;
                0
            }
            Err(_) => return_error!(-4, "Pattern not found or operation failed"),
        },
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindXrefFuncWithString(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_xref_func_with_string_rva(binary_name_str, string_str) {
        Ok(rva) => match s2binlib.rva_to_mem_address(binary_name_str, rva) {
            Ok(mem) => {
                *result = mem as *mut c_void;
                0
            }
            Err(_) => return_error!(-4, "Pattern not found or operation failed"),
        },
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindXrefFuncWithStringRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut u64,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_xref_func_with_string_rva(binary_name_str, string_str) {
        Ok(addr) => {
            *result = addr;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncWithStringRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    vfunc_rva_out: *mut u64,
) -> i32 {
    check_null!(
        this,
        binary_name,
        string,
        vtable_name_out,
        vfunc_index_out,
        vfunc_rva_out
    );
    if vtable_name_out_size == 0 {
        return_error!(-2, "invalid parameter: buffer_size is zero");
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vfunc_with_string_rva(binary_name_str, string_str) {
        Ok((vtable_info, index, rva)) => {
            let name_bytes = vtable_info.type_name.as_bytes();
            if name_bytes.len() + 1 > vtable_name_out_size {
                return_error!(-3, "buffer too small to store vtable name");
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    name_bytes.as_ptr(),
                    vtable_name_out as *mut u8,
                    name_bytes.len(),
                );
                *vtable_name_out.add(name_bytes.len()) = 0;
            }

            *vfunc_index_out = index;
            *vfunc_rva_out = rva;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindVfuncWithString(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    vtable_name_out: *mut c_char,
    vtable_name_out_size: usize,
    vfunc_index_out: *mut usize,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(
        this,
        binary_name,
        string,
        vtable_name_out,
        vfunc_index_out,
        result
    );
    if vtable_name_out_size == 0 {
        return_error!(-2, "invalid parameter: buffer_size is zero");
    }

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_vfunc_with_string_rva(binary_name_str, string_str) {
        Ok((vtable_info, index, rva)) => {
            let name_bytes = vtable_info.type_name.as_bytes();
            if name_bytes.len() + 1 > vtable_name_out_size {
                return_error!(-3, "buffer too small to store vtable name");
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    name_bytes.as_ptr(),
                    vtable_name_out as *mut u8,
                    name_bytes.len(),
                );
                *vtable_name_out.add(name_bytes.len()) = 0;
            }

            *vfunc_index_out = index;

            match s2binlib.rva_to_mem_address(binary_name_str, rva) {
                Ok(addr) => {
                    *result = addr as *mut c_void;
                    0
                }
                Err(_) => return_error!(-4, "Pattern not found or operation failed"),
            }
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindFuncWithStringRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut u64,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_func_with_string_rva(binary_name_str, string_str) {
        Ok(addr) => {
            *result = addr;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindFuncWithString(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    string: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_func_with_string(binary_name_str, string_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindNetworkvarVtableStatechangedRva(
    this: *const S2BinLib004,
    vtable_rva: u64,
    result: *mut u64,
) -> i32 {
    check_null!(this, result);

    let s2binlib = get_s2binlib!(this);

    match s2binlib.find_networkvar_vtable_statechanged_rva(vtable_rva) {
        Ok(index) => {
            *result = index;
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn FindNetworkvarVtableStatechanged(
    this: *const S2BinLib004,
    vtable_mem_address: u64,
    result: *mut u64,
) -> i32 {
    check_null!(this, result);

    let s2binlib = get_s2binlib!(this);

    match s2binlib.find_networkvar_vtable_statechanged(vtable_mem_address) {
        Ok(index) => {
            unsafe {
                *result = index;
            }
            0
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
}

unsafe extern "C" fn MakeSigRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    func_rva: u64,
    result_out: *mut c_char,
    result_out_size: usize,
) -> i32 {
    check_null!(this, binary_name, result_out);

    let s2binlib = get_s2binlib_mut!(this);

    let binary_name_str = cstr_to_str!(binary_name);
    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.make_sig_rva(binary_name_str, func_rva) {
        Ok(sig) => {
            let sig_bytes = sig.as_bytes();
            if sig_bytes.len() + 1 > result_out_size {
                return_error!(-3, "buffer too small to store signature");
            }
            unsafe {
                std::ptr::copy_nonoverlapping(sig_bytes.as_ptr(), result_out as *mut u8, sig_bytes.len());
                *result_out.add(sig_bytes.len()) = 0;
            }
            sig_bytes.len() as i32
        }
        Err(_) => return_error!(-4, "Pattern not found or operation failed"),
    }
} 

unsafe extern "C" fn FindCallWithXrefArgRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    func_start_rva: u64,
    target_rva: u64,
    result: *mut u64,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_call_with_xref_arg_rva(binary_name_str, func_start_rva, target_rva) {
        Ok(addr) => {
            *result = addr;
            0
        }
        Err(_) => return_error!(-4, "Call not found or operation failed"),
    }
}

unsafe extern "C" fn FindCallWithXrefArg(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    func_start: *mut c_void,
    target: *mut c_void,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_call_with_xref_arg(binary_name_str, func_start as u64, target as u64) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Call not found or operation failed"),
    }
}

unsafe extern "C" fn FindCallWithStringArgRva(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    func_start_rva: u64,
    string: *const c_char,
    result: *mut u64,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_call_with_string_arg_rva(binary_name_str, func_start_rva, string_str) {
        Ok(addr) => {
            *result = addr;
            0
        }
        Err(_) => return_error!(-4, "Call not found or operation failed"),
    }
}

unsafe extern "C" fn FindCallWithStringArg(
    this: *mut S2BinLib004,
    binary_name: *const c_char,
    func_start: *mut c_void,
    string: *const c_char,
    result: *mut *mut c_void,
) -> i32 {
    check_null!(this, binary_name, string, result);

    let binary_name_str = cstr_to_str!(binary_name);
    let string_str = cstr_to_str!(string);
    let s2binlib = get_s2binlib_mut!(this);

    ensure_binary_loaded!(s2binlib, binary_name_str);

    match s2binlib.find_call_with_string_arg(binary_name_str, func_start as u64, string_str) {
        Ok(addr) => {
            *result = addr as *mut c_void;
            0
        }
        Err(_) => return_error!(-4, "Call not found or operation failed"),
    }
}

unsafe extern "C" fn Destroy(this: *mut S2BinLib004) -> i32 {
    check_null!(this);

    s2binlib004_destroy(this as *mut c_void);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn s2binlib004_create() -> *mut c_void {
    let s2binlib004 = Box::new(S2BinLib004::new());
    Box::into_raw(s2binlib004) as *mut c_void
}

fn s2binlib004_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr as *mut S2BinLib004);
        }
    }
}
