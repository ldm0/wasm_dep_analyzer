// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use pretty_assertions::assert_eq;

use wasm_dep_analyzer::Export;
use wasm_dep_analyzer::ExportType;
use wasm_dep_analyzer::FunctionSignature;
use wasm_dep_analyzer::GlobalType;
use wasm_dep_analyzer::Import;
use wasm_dep_analyzer::ImportType;
use wasm_dep_analyzer::Limits;
use wasm_dep_analyzer::MemoryType;
use wasm_dep_analyzer::ParseError;
use wasm_dep_analyzer::ParseOptions;
use wasm_dep_analyzer::TableType;
use wasm_dep_analyzer::TagType;
use wasm_dep_analyzer::ValueType;
use wasm_dep_analyzer::WasmDeps;

#[test]
fn wasm_export_only() {
  // The following Rust code compiled:
  //
  // #[no_mangle]
  // pub fn add(left: usize, right: Usize) -> usize {
  //     left + right
  // }
  let input = std::fs::read("tests/testdata/export_only.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![],
      exports: vec![
        Export {
          name: "memory",
          index: 0,
          export_type: ExportType::Memory,
        },
        Export {
          name: "add",
          index: 0,
          export_type: ExportType::Function(Ok(FunctionSignature {
            params: vec![ValueType::I32, ValueType::I32],
            returns: vec![ValueType::I32],
          })),
        },
        Export {
          name: "__data_end",
          index: 1,
          export_type: ExportType::Global(Ok(GlobalType {
            value_type: ValueType::I32,
            mutability: false,
          })),
        },
        Export {
          name: "__heap_base",
          index: 2,
          export_type: ExportType::Global(Ok(GlobalType {
            value_type: ValueType::I32,
            mutability: false,
          })),
        }
      ],
    }
  );
}

#[test]
fn wasm_import_and_export() {
  // The following Rust code compiled:
  //
  // extern "C" {
  //     fn get_random_value() -> usize;
  // }

  // #[no_mangle]
  // pub fn add(left: usize) -> usize {
  //     left + unsafe { get_random_value() }
  // }
  let input = std::fs::read("tests/testdata/import_export.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "get_random_value",
        module: "env",
        import_type: ImportType::Function(0),
      }],
      exports: vec![
        Export {
          name: "memory",
          index: 0,
          export_type: ExportType::Memory,
        },
        Export {
          name: "add",
          index: 1,
          export_type: ExportType::Function(Ok(FunctionSignature {
            params: vec![ValueType::I32],
            returns: vec![ValueType::I32],
          })),
        },
        Export {
          name: "__data_end",
          index: 1,
          export_type: ExportType::Global(Ok(GlobalType {
            value_type: ValueType::I32,
            mutability: false,
          })),
        },
        Export {
          name: "__heap_base",
          index: 2,
          export_type: ExportType::Global(Ok(GlobalType {
            value_type: ValueType::I32,
            mutability: false,
          })),
        }
      ],
    }
  );
}

#[test]
fn wasm_import_module() {
  let input = std::fs::read("tests/testdata/import_module.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "add",
        module: "./import_inner.mjs",
        import_type: ImportType::Function(0),
      }],
      exports: vec![Export {
        name: "exported_add",
        index: 1,
        export_type: ExportType::Function(Ok(FunctionSignature {
          params: vec![], // this module actually just adds two constants
          returns: vec![ValueType::I32],
        })),
      }],
    }
  );
}

#[test]
fn wasm_mutable_global_import() {
  // from wat2wasm "mutable globals" example
  // (module
  // (import "env" "g" (global (mut i32)))
  // (func (export "f")
  //  i32.const 100
  //  global.set 0))
  let input =
    std::fs::read("tests/testdata/import_mutable_global.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "g",
        module: "env",
        import_type: ImportType::Global(GlobalType {
          value_type: ValueType::I32,
          mutability: true,
        }),
      }],
      exports: vec![Export {
        name: "f",
        index: 0,
        export_type: ExportType::Function(Ok(FunctionSignature {
          params: vec![],
          returns: vec![],
        })),
      }],
    }
  );
}

#[test]
fn wasm_import_table() {
  // (module
  //   ;; Import a table named "tbl" from a module named "js".
  //   ;; The table is of type "funcref" with initial size 2 and no maximum size.
  //   (import "js" "tbl" (table 2 funcref))
  // )
  let input = std::fs::read("tests/testdata/import_table.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "table",
        module: "env",
        import_type: ImportType::Table(TableType {
          element_type: 112,
          limits: Limits {
            initial: 2,
            maximum: None,
          },
        }),
      }],
      exports: vec![],
    }
  );
}

#[test]
fn wasm_import_table_externref() {
  let input = [
    0x00, 0x61, 0x73, 0x6d, // magic
    0x01, 0x00, 0x00, 0x00, // version
    0x02, 0x0c, // import section
    0x01, // import count
    0x02, b'j', b's', // module
    0x03, b't', b'b', b'l', // name
    0x01, // table import
    0x6f, // externref
    0x00, // limits without maximum
    0x02, // initial
  ];
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "tbl",
        module: "js",
        import_type: ImportType::Table(TableType {
          element_type: 0x6f,
          limits: Limits {
            initial: 2,
            maximum: None,
          },
        }),
      }],
      exports: vec![],
    }
  );
}

#[test]
fn wasm_import_table_ref_null_extern() {
  let input = [
    0x00, 0x61, 0x73, 0x6d, // magic
    0x01, 0x00, 0x00, 0x00, // version
    0x02, 0x0d, // import section
    0x01, // import count
    0x02, b'j', b's', // module
    0x03, b't', b'b', b'l', // name
    0x01, // table import
    0x63, 0x6f, // ref null extern
    0x00, // limits without maximum
    0x02, // initial
  ];
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "tbl",
        module: "js",
        import_type: ImportType::Table(TableType {
          element_type: 0x63,
          limits: Limits {
            initial: 2,
            maximum: None,
          },
        }),
      }],
      exports: vec![],
    }
  );
}

#[test]
fn wasm_import_shared_memory_limits_with_maximum() {
  let input = [
    0x00, 0x61, 0x73, 0x6d, // magic
    0x01, 0x00, 0x00, 0x00, // version
    0x02, 0x0c, // import section
    0x01, // import count
    0x02, b'j', b's', // module
    0x03, b'm', b'e', b'm', // name
    0x02, // memory import
    0x03, // limits with maximum + shared
    0x01, // initial
    0x02, // maximum
  ];
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "mem",
        module: "js",
        import_type: ImportType::Memory(MemoryType {
          limits: Limits {
            initial: 1,
            maximum: Some(2),
          },
        }),
      }],
      exports: vec![],
    }
  );
}

#[test]
fn wasm_import_tag() {
  // (module
  //   ;; Import an exception tag
  //   (import "env" "exception_tag" (tag (type 0)))

  //   ;; Define an exception type (a list of value types that the exception can carry)
  //   (type (func (param i32)))

  //   ;; Define a tag that uses the above exception type
  //   (tag (type 0))

  //   ;; Export the defined tag
  //   (export "exported_tag" (tag 0))
  // )
  let input = std::fs::read("tests/testdata/import_tag.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "exception_tag",
        module: "env",
        import_type: ImportType::Tag(TagType {
          kind: 0,
          type_index: 0,
        }),
      }],
      exports: vec![Export {
        name: "exported_tag",
        index: 0,
        export_type: ExportType::Tag,
      }],
    }
  );
}

#[test]
fn wasm_export_memory() {
  // (module
  //   ;; Define a memory with an initial size of 1 page (64KiB)
  //   ;; and a maximum size of 10 pages (640KiB).
  //   (memory (export "mem") 1 10)
  // )
  let input = std::fs::read("tests/testdata/export_memory.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![],
      exports: vec![Export {
        name: "mem",
        index: 0,
        export_type: ExportType::Memory,
      }],
    }
  );
}

#[test]
fn wasm_export_mutable_global() {
  // (module
  //   ;; Define a mutable global of type i32 initialized to 42
  //   (global $myGlobal (export "myExportedGlobal") (mut i32) (i32.const 42))
  // )
  let input =
    std::fs::read("tests/testdata/export_mutable_global.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![],
      exports: vec![Export {
        name: "myExportedGlobal",
        index: 0,
        export_type: ExportType::Global(Ok(GlobalType {
          value_type: ValueType::I32,
          mutability: true,
        })),
      }],
    }
  );
}

#[test]
fn wasm_export_const_global() {
  // (module
  //   ;; Define a constant global of type i32 initialized to 42
  //   (global $myGlobal (export "myExportedGlobal") i32 (i32.const 42))
  // )
  let input = std::fs::read("tests/testdata/export_const_global.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![],
      exports: vec![Export {
        name: "myExportedGlobal",
        index: 0,
        export_type: ExportType::Global(Ok(GlobalType {
          value_type: ValueType::I32,
          mutability: false,
        })),
      }],
    }
  );
}

#[test]
fn wasm_export_imported_and_local_globals_use_global_index_space() {
  let input = [
    0x00, 0x61, 0x73, 0x6d, // magic
    0x01, 0x00, 0x00, 0x00, // version
    0x02, 0x0a, // import section
    0x01, // import count
    0x03, b'e', b'n', b'v', // module
    0x01, b'g', // name
    0x03, // global import
    0x7f, // i32
    0x01, // mutable
    0x06, 0x06, // global section
    0x01, // global count
    0x7e, // i64
    0x00, // immutable
    0x42, 0x00, 0x0b, // i64.const 0; end
    0x07, 0x14, // export section
    0x02, // export count
    0x08, b'i', b'm', b'p', b'o', b'r', b't', b'e', b'd', // name
    0x03, // global
    0x00, // imported global index
    0x05, b'l', b'o', b'c', b'a', b'l', // name
    0x03, // global
    0x01, // local global index
  ];
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "g",
        module: "env",
        import_type: ImportType::Global(GlobalType {
          value_type: ValueType::I32,
          mutability: true,
        }),
      }],
      exports: vec![
        Export {
          name: "imported",
          index: 0,
          export_type: ExportType::Global(Ok(GlobalType {
            value_type: ValueType::I32,
            mutability: true,
          })),
        },
        Export {
          name: "local",
          index: 1,
          export_type: ExportType::Global(Ok(GlobalType {
            value_type: ValueType::I64,
            mutability: false,
          })),
        },
      ],
    }
  );
}

#[test]
fn wasm_export_v128_global_type() {
  let input = [
    0x00, 0x61, 0x73, 0x6d, // magic
    0x01, 0x00, 0x00, 0x00, // version
    0x06, 0x16, // global section
    0x01, // global count
    0x7b, // v128
    0x00, // immutable
    0xfd, 0x0c, // v128.const
    0x00, 0x00, 0x00, 0x00, // lane bytes
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x0b, // end
    0x07, 0x05, // export section
    0x01, // export count
    0x01, b'v', // name
    0x03, // global
    0x00, // index
  ];
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![],
      exports: vec![Export {
        name: "v",
        index: 0,
        export_type: ExportType::Global(Ok(GlobalType {
          value_type: ValueType::V128,
          mutability: false,
        })),
      }],
    }
  );
}

#[test]
fn wasm_export_imported_func() {
  // (module
  //   ;; Import a function named 'external_func' from the 'env' module.
  //   ;; The function takes two i32 integers and returns an i32 integer.
  //   (import "env" "external_func" (func $imported_func (param i32 i32) (result i32)))

  //   ;; Export the imported function under the name 'exported_func'.
  //   (export "exported_func" (func $imported_func))
  // )
  let input =
    std::fs::read("tests/testdata/export_imported_func.wasm").unwrap();
  let module = WasmDeps::parse(&input, ParseOptions::default()).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "external_func",
        module: "env",
        import_type: ImportType::Function(0),
      }],
      exports: vec![Export {
        name: "exported_func",
        index: 0,
        export_type: ExportType::Function(Ok(FunctionSignature {
          params: vec![ValueType::I32, ValueType::I32],
          returns: vec![ValueType::I32],
        })),
      }],
    }
  );
}

#[test]
fn wasm_skip_types() {
  // function
  let input =
    std::fs::read("tests/testdata/export_imported_func.wasm").unwrap();
  let module =
    WasmDeps::parse(&input, ParseOptions { skip_types: true }).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![Import {
        name: "external_func",
        module: "env",
        import_type: ImportType::Function(0),
      }],
      exports: vec![Export {
        name: "exported_func",
        index: 0,
        // won't be resolved
        export_type: ExportType::Function(Err(
          ParseError::UnresolvedExportType
        )),
      }],
    }
  );

  // global
  let input = std::fs::read("tests/testdata/export_const_global.wasm").unwrap();
  let module =
    WasmDeps::parse(&input, ParseOptions { skip_types: true }).unwrap();
  assert_eq!(
    module,
    WasmDeps {
      imports: vec![],
      exports: vec![Export {
        name: "myExportedGlobal",
        index: 0,
        export_type: ExportType::Global(Err(ParseError::UnresolvedExportType)),
      }],
    }
  );
}
