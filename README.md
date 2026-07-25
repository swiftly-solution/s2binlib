# S2BinLib

A static library that helps resolving memory from binary file and map to absolute memory address, targeting source 2 game engine.

This library mainly read data from original binary file instead of from memory, which solves the issue that utilities like patter scan may fail if the memory is modified or hooked.

## Features

- SIMD Optimized pattern scan
- Find vtable by name
- Find vfunc by VA and memory address
- Get all xrefs to a specific VA
- Find string
- Find export
- Find symbol
- Get module base address
- Install trampoline to vtable with enough bytes padded with NOP (for safetyhook to hook empty virtual function)
- Find all CEmbeddedNetworkVar NetworkStateChanged function index
- Follow xref safely
- Dump vtables
- Get runtime object rtti info (name, base classes)
- Dump strings
- Dump entity classes (dumper)
- Dump vtable diffs (dumper)
- Dump game system overrides (dumper)
- Dump network var vtables (dumper)


## Project Layout

- `s2binlib`: core Rust library crate exposing safe APIs.
- `s2binlib_binding`: C ABI wrapper crate that links to `s2binlib` and produces the `s2binlib` DLL/LIB artifacts.

## Compiling

Prerequisites:
- Rust development environment

To build the C bindings (emitting `s2binlib.dll`, `libs2binlib.a`, etc.):
```
cargo build -p s2binlib_binding --release
```

To build only the Rust core crate:
```
cargo build -p s2binlib --release
```

### Debug Mode for C Bindings

To enable debug output for all C binding errors, compile the binding crate with the `debug_c_bindings` feature:
```
cargo build -p s2binlib_binding --release --features debug_c_bindings
```

When enabled, all error returns in C bindings will print detailed debug information to stdout, including:
- Error code
- Error message
- File name and line number where the error occurred

This is useful for debugging integration issues with the C API.

## Linking and building

On windows, if you are seeing linking error like `"__imp_NtReadFile"`, you also need to link `kernel32.dll` and `ntdll.dll`.

## C API

S2BinLib provides two C API styles:

### 1. Global Singleton API (Recommended for simple use cases, static linking)

The global singleton API (`s2binlib.h`) provides a thread-safe, easy-to-use interface with all functions prefixed with `s2binlib_`:

```c
#include <s2binlib.h>

int main() {
  // Initialize the global instance (game type is "csgo" for CS2)
  s2binlib_initialize("/home/csgo/cs2server/game", "csgo");

  // If you are using metamod, relocate these modules because they're modified
  GET_V_IFACE_ANY(GetServerFactory, g_pSource2Server, ISource2Server, SOURCE2SERVER_INTERFACE_VERSION);
  GET_V_IFACE_CURRENT(GetEngineFactory, g_pEngineServer2, IVEngineServer2, SOURCE2ENGINETOSERVER_INTERFACE_VERSION);
  s2binlib_set_module_base_from_pointer("server", g_pSource2Server);
  s2binlib_set_module_base_from_pointer("engine2", g_pEngineServer2);

  // Pattern scan
  void* result;
  s2binlib_pattern_scan("server", "01 02 03 AA BB CC ? ? DD", &result);

  // Find a vtable
  void* vtable_addr;
  s2binlib_find_vtable("server", "CBaseEntity", &vtable_addr);

  // Free after use, this will only release the file bytes in memory
  // Dumped xref and other information will still be cached
  s2binlib_unload_all_binaries();

  // Clean up the global instance
  s2binlib_destroy();
}
```

**Features:**
- Thread-safe global singleton
- Simple API with no manual instance management
- All functions use snake_case naming with `s2binlib_` prefix
- Return values: 0 for success, negative for errors (-1: not initialized, -2: invalid parameter, -3: operation failed, -4: not found, -99: mutex error)

### 2. Object-Oriented API (For version control)

The object-oriented API (`s2binlib001.h`) allows multiple instances with explicit lifecycle control:

```cpp
#include <s2binlib001.h>

int main() {
  // Create instance manually
  void* s2binlib_ptr = s2binlib001_create();
  S2BinLib001* s2binlib = (S2BinLib001*)s2binlib_ptr;

  // Initialize through vtable
  s2binlib->Initialize("/home/csgo/cs2server/game", "csgo");

  // Use vtable methods
  void* result;
  s2binlib->PatternScan("server", "01 02 03 AA BB CC ? ? DD", &result);

  // Destroy instance
  s2binlib->Destroy();
}
```

### 3. Dynamic Loading
Or use the `CreateInterface` function for dynamic loading:

```cpp
// windows example
S2CreateInterfaceFn createInterface = (S2CreateInterfaceFn)GetProcAddress(hDll, "S2BinLib_CreateInterface");
auto s2binlib = createInterface(S2BINLIB_INTERFACE_NAME);
```