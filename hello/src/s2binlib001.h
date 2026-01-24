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

#ifndef _s2binlib_s2binlib001_h
#define _s2binlib_s2binlib001_h

#pragma once

#include <cstdint>
#include <cstddef>

#ifdef __cplusplus
extern "C"
{
#endif

#define S2BINLIB_INTERFACE_NAME "S2BINLIB001"

    /// Forward declaration
    struct S2BinLib001;

    /// Callback function type for pattern_scan_all functions
    /// @param index The index of the current match (0-based)
    /// @param address The found address (RVA or memory address depending on the function)
    /// @param user_data User-provided data pointer
    /// @return true to stop searching, false to continue searching for more matches
    typedef bool (*PatternScanCallback)(size_t index, void *address, void *user_data);

    /// Create a new S2BinLib001 instance
    /// @param interface_name Interface name to create
    /// @return Pointer to the created instance, or nullptr on failure
    typedef S2BinLib001 *(*S2CreateInterfaceFn)(const char *interface_name);

#ifdef __cplusplus
}

/// S2BinLib001 class - Interface version 001
/// This class provides access to S2BinLib functionality through a virtual table interface
class S2BinLib001
{
public:
    /// Initialize with auto-detected operating system
    /// @param game_path Path to the game directory (null-terminated C string)
    /// @param game_type Game type identifier (null-terminated C string)
    /// @return 0 on success, negative error code on failure
    virtual int Initialize(const char *game_path, const char *game_type) = 0;

    /// Initialize with explicit operating system parameter
    /// @param game_path Path to the game directory (null-terminated C string)
    /// @param game_type Game type identifier (null-terminated C string)
    /// @param os Operating system ("windows" or "linux") (null-terminated C string)
    /// @return 0 on success, negative error code on failure
    virtual int InitializeWithOs(const char *game_path, const char *game_type, const char *os) = 0;

    /// Scan for a pattern in the specified binary and return its memory address
    /// @param binary_name Name of the binary to scan (e.g., "server", "client")
    /// @param pattern Pattern string with wildcards (e.g., "48 89 5C 24 ? 48 89 74 24 ?")
    /// @param result Pointer to store the resulting address
    /// @return 0 on success, negative error code on failure
    virtual int PatternScan(const char *binary_name, const char *pattern, void **result) = 0;

    /// Find a vtable by class name and return its memory address
    /// @param binary_name Name of the binary to search (e.g., "server", "client")
    /// @param vtable_name Class name to search for
    /// @param result Pointer to store the resulting vtable address
    /// @return 0 on success, negative error code on failure
    virtual int FindVtable(const char *binary_name, const char *vtable_name, void **result) = 0;

    /// Find a symbol by name and return its memory address
    /// @param binary_name Name of the binary to search
    /// @param symbol_name Symbol name to search for
    /// @param result Pointer to store the resulting symbol address
    /// @return 0 on success, negative error code on failure
    virtual int FindSymbol(const char *binary_name, const char *symbol_name, void **result) = 0;

    /// Set module base address from a pointer inside the module
    /// @param binary_name Name of the binary
    /// @param pointer Pointer inside the specified module
    /// @return 0 on success, negative error code on failure
    virtual int SetModuleBaseFromPointer(const char *binary_name, void *pointer) = 0;

    /// Clear manually set base address for a module
    /// @param binary_name Name of the binary
    /// @return 0 on success, negative error code on failure
    virtual int ClearModuleBaseAddress(const char *binary_name) = 0;

    /// Set a custom binary path for a specific binary and operating system
    /// @param binary_name Name of the binary
    /// @param path The custom file path to the binary
    /// @param os Operating system identifier ("windows" or "linux")
    /// @return 0 on success, negative error code on failure
    virtual int SetCustomBinaryPath(const char *binary_name, const char *path, const char *os) = 0;

    /// Get the module base address
    /// @param binary_name Name of the binary
    /// @param result Pointer to store the resulting base address
    /// @return 0 on success, negative error code on failure
    virtual int GetModuleBaseAddress(const char *binary_name, void **result) const = 0;

    /// Check if a binary is already loaded
    /// @param binary_name Name of the binary to check
    /// @return 1 if loaded, 0 if not loaded, negative error code on failure
    virtual int IsBinaryLoaded(const char *binary_name) const = 0;

    /// Load a binary into memory
    /// @param binary_name Name of the binary to load
    /// @return 0 on success, negative error code on failure
    virtual int LoadBinary(const char *binary_name) = 0;

    /// Get the full path to a binary file
    /// @param binary_name Name of the binary
    /// @param buffer Buffer to store the path string
    /// @param buffer_size Size of the buffer
    /// @return 0 on success, negative error code on failure
    virtual int GetBinaryPath(const char *binary_name, char *buffer, size_t buffer_size) const = 0;

    /// Find a vtable by class name and return its relative virtual address
    /// @param binary_name Name of the binary to search
    /// @param vtable_name Class name to search for
    /// @param result Pointer to store the resulting vtable RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindVtableRva(const char *binary_name, const char *vtable_name, void **result) = 0;

    /// Find a vtable by mangled name and return its relative virtual address
    /// @param binary_name Name of the binary to search
    /// @param vtable_name Mangled RTTI name to search for
    /// @param result Pointer to store the resulting vtable RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindVtableMangledRva(const char *binary_name, const char *vtable_name, void **result) = 0;

    /// Find a vtable by mangled name and return its runtime memory address
    /// @param binary_name Name of the binary to search
    /// @param vtable_name Mangled RTTI name to search for
    /// @param result Pointer to store the resulting vtable memory address
    /// @return 0 on success, negative error code on failure
    virtual int FindVtableMangled(const char *binary_name, const char *vtable_name, void **result) = 0;

    /// Find a nested vtable (2 levels) by class names and return its RVA
    /// @param binary_name Name of the binary to search
    /// @param class1_name Outer class name
    /// @param class2_name Inner/nested class name
    /// @param result Pointer to store the resulting vtable RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindVtableNested2Rva(const char *binary_name, const char *class1_name, const char *class2_name, void **result) = 0;

    /// Find a nested vtable (2 levels) by class names and return its memory address
    /// @param binary_name Name of the binary to search
    /// @param class1_name Outer class name
    /// @param class2_name Inner/nested class name
    /// @param result Pointer to store the resulting vtable memory address
    /// @return 0 on success, negative error code on failure
    virtual int FindVtableNested2(const char *binary_name, const char *class1_name, const char *class2_name, void **result) = 0;

    /// Get the number of virtual functions in a vtable
    /// @param binary_name Name of the binary to search
    /// @param vtable_name Name of the vtable/class
    /// @param result Pointer to store the resulting vfunc count
    /// @return 0 on success, negative error code on failure
    virtual int GetVtableVfuncCount(const char *binary_name, const char *vtable_name, size_t *result) = 0;

    /// Get the number of virtual functions in a vtable by RVA
    /// @param binary_name Name of the binary
    /// @param vtable_rva Virtual address of the vtable
    /// @param result Pointer to store the resulting vfunc count
    /// @return 0 on success, negative error code on failure
    virtual int GetVtableVfuncCountByRva(const char *binary_name, uint64_t vtable_rva, size_t *result) = 0;

    /// Scan for a pattern and return its relative virtual address
    /// @param binary_name Name of the binary to scan
    /// @param pattern Pattern string with wildcards
    /// @param result Pointer to store the resulting RVA
    /// @return 0 on success, negative error code on failure
    virtual int PatternScanRva(const char *binary_name, const char *pattern, void **result) = 0;

    /// Find all occurrences of a pattern and return their RVAs via callback
    /// @param binary_name Name of the binary to scan
    /// @param pattern Byte pattern to search for
    /// @param callback Function pointer that will be called for each match
    /// @param user_data User-provided pointer passed to each callback invocation
    /// @return 0 on success, negative error code on failure
    virtual int PatternScanAllRva(const char *binary_name, const char *pattern, PatternScanCallback callback, void *user_data) = 0;

    /// Find all occurrences of a pattern and return their memory addresses via callback
    /// @param binary_name Name of the binary to scan
    /// @param pattern Byte pattern to search for
    /// @param callback Function pointer that will be called for each match
    /// @param user_data User-provided pointer passed to each callback invocation
    /// @return 0 on success, negative error code on failure
    virtual int PatternScanAll(const char *binary_name, const char *pattern, PatternScanCallback callback, void *user_data) = 0;

    /// Find an exported symbol and return its relative virtual address
    /// @param binary_name Name of the binary to search
    /// @param export_name Export name to search for
    /// @param result Pointer to store the resulting RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindExportRva(const char *binary_name, const char *export_name, void **result) = 0;

    /// Find an exported symbol and return its runtime memory address
    /// @param binary_name Name of the binary to search
    /// @param export_name Export name to search for
    /// @param result Pointer to store the resulting memory address
    /// @return 0 on success, negative error code on failure
    virtual int FindExport(const char *binary_name, const char *export_name, void **result) = 0;

    /// Find a symbol and return its relative virtual address
    /// @param binary_name Name of the binary to search
    /// @param symbol_name Symbol name to search for
    /// @param result Pointer to store the resulting RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindSymbolRva(const char *binary_name, const char *symbol_name, void **result) = 0;

    /// Read bytes from binary at a file offset
    /// @param binary_name Name of the binary to read from
    /// @param file_offset File offset to read from
    /// @param buffer Buffer to store the read bytes
    /// @param buffer_size Size of the buffer (number of bytes to read)
    /// @return 0 on success, negative error code on failure
    virtual int ReadByFileOffset(const char *binary_name, uint64_t file_offset, uint8_t *buffer, size_t buffer_size) = 0;

    /// Read bytes from binary at a relative virtual address
    /// @param binary_name Name of the binary to read from
    /// @param rva Virtual address to read from
    /// @param buffer Buffer to store the read bytes
    /// @param buffer_size Size of the buffer (number of bytes to read)
    /// @return 0 on success, negative error code on failure
    virtual int ReadByRva(const char *binary_name, uint64_t rva, uint8_t *buffer, size_t buffer_size) = 0;

    /// Read bytes from binary at a runtime memory address
    /// @param binary_name Name of the binary to read from
    /// @param mem_address Runtime memory address to read from
    /// @param buffer Buffer to store the read bytes
    /// @param buffer_size Size of the buffer (number of bytes to read)
    /// @return 0 on success, negative error code on failure
    virtual int ReadByMemAddress(const char *binary_name, uint64_t mem_address, uint8_t *buffer, size_t buffer_size) = 0;

    /// Find a virtual function by vtable name and index, return RVA
    /// @param binary_name Name of the binary to search
    /// @param vtable_name Class name whose vtable to search for
    /// @param vfunc_index Index of the virtual function in the vtable (0-based)
    /// @param result Pointer to store the resulting RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindVfuncByVtbnameRva(const char *binary_name, const char *vtable_name, size_t vfunc_index, void **result) = 0;

    /// Find a virtual function by vtable name and index, return memory address
    /// @param binary_name Name of the binary to search
    /// @param vtable_name Class name whose vtable to search for
    /// @param vfunc_index Index of the virtual function in the vtable (0-based)
    /// @param result Pointer to store the resulting memory address
    /// @return 0 on success, negative error code on failure
    virtual int FindVfuncByVtbname(const char *binary_name, const char *vtable_name, size_t vfunc_index, void **result) = 0;

    /// Find a virtual function by vtable pointer and index, return RVA
    /// @param vtable_ptr Runtime pointer to the vtable
    /// @param vfunc_index Index of the virtual function in the vtable (0-based)
    /// @param result Pointer to store the resulting RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindVfuncByVtbptrRva(void *vtable_ptr, size_t vfunc_index, void **result) const = 0;

    /// Find a virtual function by vtable pointer and index, return memory address
    /// @param vtable_ptr Runtime pointer to the vtable
    /// @param vfunc_index Index of the virtual function in the vtable (0-based)
    /// @param result Pointer to store the resulting memory address
    /// @return 0 on success, negative error code on failure
    virtual int FindVfuncByVtbptr(void *vtable_ptr, size_t vfunc_index, void **result) const = 0;

    /// Get the vtable name from an object pointer
    /// @param object_ptr Pointer to the object
    /// @param buffer Buffer to store the vtable name
    /// @param buffer_size Size of the buffer
    /// @return 0 on success, negative error code on failure
    virtual int GetObjectPtrVtableName(const void *object_ptr, char *buffer, size_t buffer_size) const = 0;

    /// Check if an object pointer has a valid vtable
    /// @param object_ptr Pointer to the object
    /// @return 1 if has vtable, 0 if not, negative error code on failure
    virtual int ObjectPtrHasVtable(const void *object_ptr) const = 0;

    /// Check if an object has a specific base class
    /// @param object_ptr Pointer to the object
    /// @param base_class_name Name of the base class to check
    /// @return 1 if has base class, 0 if not, negative error code on failure
    virtual int ObjectPtrHasBaseClass(const void *object_ptr, const char *base_class_name) const = 0;

    /// Find a string in the binary and return its relative virtual address
    /// @param binary_name Name of the binary to search
    /// @param string String to search for
    /// @param result Pointer to store the resulting RVA
    /// @return 0 on success, negative error code on failure
    virtual int FindStringRva(const char *binary_name, const char *string, void **result) = 0;

    /// Find a string in the binary and return its runtime memory address
    /// @param binary_name Name of the binary to search
    /// @param string String to search for
    /// @param result Pointer to store the resulting memory address
    /// @return 0 on success, negative error code on failure
    virtual int FindString(const char *binary_name, const char *string, void **result) = 0;

    /// Dump and cache all cross-references in a binary
    /// @param binary_name Name of the binary to analyze
    /// @return 0 on success, negative error code on failure
    virtual int DumpXrefs(const char *binary_name) = 0;

    /// Get the count of cached cross-references for a target RVA
    /// @param binary_name Name of the binary
    /// @param target_rva The target relative virtual address
    /// @return Non-negative count of xrefs, negative error code on failure
    virtual int GetXrefsCount(const char *binary_name, void *target_rva) const = 0;

    /// Get cached cross-references for a target RVA into a buffer
    /// @param binary_name Name of the binary
    /// @param target_rva The target relative virtual address
    /// @param buffer Buffer to store the xref addresses
    /// @param buffer_size Size of the buffer (number of void* elements)
    /// @return Non-negative count of xrefs written, negative error code on failure
    virtual int GetXrefsCached(const char *binary_name, void *target_rva, void **buffer, size_t buffer_size) const = 0;

    /// Unload a specific binary from memory
    /// @param binary_name Name of the binary to unload
    /// @return 0 on success, negative error code on failure
    virtual int UnloadBinary(const char *binary_name) = 0;

    /// Unload all binaries from memory
    /// @return 0 on success, negative error code on failure
    virtual int UnloadAllBinaries() = 0;

    /// Install a JIT trampoline at a memory address
    /// @param mem_address Runtime memory address where to install the trampoline
    /// @param trampoline_address_out Pointer to store the trampoline address
    /// @return 0 on success, negative error code on failure
    virtual int InstallTrampoline(void *mem_address, void **trampoline_address_out) = 0;

    /// Follow cross-reference from memory address to memory address
    /// @param mem_address Runtime memory address to analyze
    /// @param target_address_out Pointer to store the target address
    /// @return 0 on success, negative error code on failure
    virtual int FollowXrefMemToMem(const void *mem_address, void **target_address_out) const = 0;

    /// Follow cross-reference from RVA to memory address
    /// @param binary_name Name of the binary
    /// @param rva Virtual address to analyze
    /// @param target_address_out Pointer to store the target memory address
    /// @return 0 on success, negative error code on failure
    virtual int FollowXrefRvaToMem(const char *binary_name, uint64_t rva, void **target_address_out) = 0;

    /// Follow cross-reference from RVA to RVA
    /// @param binary_name Name of the binary
    /// @param rva Virtual address to analyze
    /// @param target_rva_out Pointer to store the target RVA
    /// @return 0 on success, negative error code on failure
    virtual int FollowXrefRvaToRva(const char *binary_name, uint64_t rva, uint64_t *target_rva_out) = 0;

    /// Find the NetworkVar_StateChanged vtable index by RVA
    /// @param vtable_rva Virtual address of the vtable to analyze
    /// @param result Pointer to store the resulting index
    /// @return 0 on success, negative error code on failure
    virtual int FindNetworkvarVtableStatechangedRva(uint64_t vtable_rva, uint64_t *result) const = 0;

    /// Find the NetworkVar_StateChanged vtable index by memory address
    /// @param vtable_mem_address Runtime memory address of the vtable
    /// @param result Pointer to store the resulting index
    /// @return 0 on success, negative error code on failure
    virtual int FindNetworkvarVtableStatechanged(uint64_t vtable_mem_address, uint64_t *result) const = 0;

    /// Destroy the S2BinLib001 instance
    /// @return 0 on success, negative error code on failure
    virtual int Destroy() = 0;
};

#endif // __cplusplus

#endif // _s2binlib_s2binlib001_h