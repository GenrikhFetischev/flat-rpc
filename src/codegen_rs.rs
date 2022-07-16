use convert_case::{Case, Casing};
use quote::{quote, format_ident};
use crate::parser::{DeclType, Statements};
use crate::ir::{TableDeclaration, RpcDeclaration, Type, ValueType, StructDeclaration};


pub fn generate_rust_server_side_code(statements: &Statements) -> String {
  let mut generated = vec![generate_header(statements)];

  for struct_def in statements.struct_declaration.values() {
    generated.push(struct_def.to_rs_string(statements));
    generated.push(struct_def.generate_into_protocol_struct_impl());
  };
  for interface in statements.table_declaration.values() {
    generated.push(interface.to_rs_string(statements));
    generated.push(interface.generate_into_offset_impl(statements));
    generated.push(interface.generate_into_byte_vec_impl(statements));
  }
  for rpc in statements.rpc_declarations.values() {
    generated.push(rpc.to_rs_string(statements));
  }
  generated.push(generate_process_request_fn(statements));
  generated.join("\n")
}


pub fn generate_header(statements: &Statements) -> String {
  let mut imports = vec![];

  for table_declaration in statements.table_declaration.values() {
    imports.push(table_declaration.name.to_owned());
    imports.push(format!("{}Args", table_declaration.name));
    imports.push(format!("{}T", table_declaration.name));
  }

  for struct_declaration in statements.struct_declaration.values() {
    imports.push(struct_declaration.name.to_owned());
    imports.push(format!("{}T", struct_declaration.name));
  }

  for enum_declaration in statements.enum_declarations.values() {
    imports.push(enum_declaration.name.to_owned());
  }


  for name in statements.unions.keys() {
    imports.push(name.to_owned());
  }


  format!(r#"
use tokio::task::JoinHandle;
pub use crate::schema_generated::protocol::{{ {}, root_as_root_table }};
use flatbuffers::{{FlatBufferBuilder, WIPOffset, UnionWIPOffset}};
pub trait IntoOffset<T: 'static> {{
  fn into_offset(self, builder: &mut FlatBufferBuilder<'static>) -> WIPOffset<T>;
}}

pub fn into_root_type(binary: &[u8]) -> RootTable {{
  let root_type = root_as_root_table(binary);
  if root_type.is_err() {{
    println!("Protocol error: can't parse binary as a root type");
  }}
  root_type.unwrap()
}}
"#, imports.join(", "))
}

pub fn generate_process_request_fn(statements: &Statements) -> String {
  let mut imp = vec![
    "pub async fn process_request<RequestHandlerStruct: RequestHandler>(buffer: Vec<u8>) -> Vec<u8> {".to_owned(),
    "let root_type = into_root_type(&buffer);".to_owned(),
    "match root_type.payload_type() {".to_owned(),
  ];


  for rpc in statements.rpc_declarations.values() {
    for method in rpc.methods.values() {
      imp.push(format!(r#"AvailableItems::{} => RequestHandlerStruct::{}(buffer).await.expect("error while handling {}").into(),"#,
                       method.input,
                       method.name.to_case(Case::Snake),
                       method.name.to_case(Case::Snake)
      ))
    }
  }


  imp.push(r#"
  unknown_variant => {
       let msg = format!("UNKNOWN PAYLOAD TYPE {:?}", unknown_variant);
       panic!("{}", msg);
     }

  "#.to_owned());
  imp.push("}\n}".to_owned());

  imp.join("\n")
}

pub trait ToRsStatement {
  fn to_rs_string(&self, statements: &Statements) -> String;
}

pub trait ToTsString {
  fn to_ts_string(&self, statements: &Statements) -> String;
}

pub trait GenerateIntoProtocolStructImpl {
  fn generate_into_protocol_struct_impl(&self) -> String;
}

pub trait GenerateIntoOffsetImpl {
  fn generate_into_offset_impl(&self, statements: &Statements) -> String;
}

pub trait GenerateIntoByteVecImpl {
  fn generate_into_byte_vec_impl(&self, statements: &Statements) -> String;
}

impl ToRsStatement for ValueType {
  fn to_rs_string(&self, _: &Statements) -> String {
    match self {
      ValueType::Bool => "bool".to_owned(),
      ValueType::Byte => "i8".to_owned(),
      ValueType::Ubyte => "u8".to_owned(),
      ValueType::Short => "i16".to_owned(),
      ValueType::Ushort => "u16".to_owned(),
      ValueType::Int => "i32".to_owned(),
      ValueType::Uint => "u32".to_owned(),
      ValueType::Float => "f32".to_owned(),
      ValueType::Long => "i64".to_owned(),
      ValueType::Ulong => "u64".to_owned(),
      ValueType::Double => "f64".to_owned(),
      ValueType::Int8 => "i8".to_owned(),
      ValueType::Uint8 => "u8".to_owned(),
      ValueType::Int16 => "i16".to_owned(),
      ValueType::Uint16 => "u16".to_owned(),
      ValueType::Int32 => "i32".to_owned(),
      ValueType::Uint32 => "u32".to_owned(),
      ValueType::Int64 => "i64".to_owned(),
      ValueType::Uint64 => "u64".to_owned(),
      ValueType::Float32 => "f32".to_owned(),
      ValueType::Float64 => "f64".to_owned(),
      ValueType::String => "String".to_owned(),
      ValueType::CompoundType(compound) => {
        format!("{}Content", compound)
      }
    }
  }
}

impl ToRsStatement for Type {
  fn to_rs_string(&self, statements: &Statements) -> String {
    if self.is_array {
      format!("Vec<{}>", self.value_type.to_rs_string(statements))
    } else {
      self.value_type.to_rs_string(statements)
    }
  }
}

impl GenerateIntoOffsetImpl for TableDeclaration {
  fn generate_into_offset_impl(&self, statements: &Statements) -> String {
    let mut imp = vec![
      format!("impl IntoOffset<{}<'static>> for {}Content {{", &self.name, &self.name),
      format!("fn into_offset(self, builder: &mut FlatBufferBuilder<'static>) -> WIPOffset<{}<'static>> {{", &self.name),
    ];

    let mut field_offsets = vec![];
    let mut straight_fields = vec![];
    let mut unions = vec![];
    let mut structs = vec![];
    let mut enums = vec![];

    for (field_name, field_type) in self.fields.iter() {
      match (&field_type.value_type, field_type.is_array) {
        (ValueType::CompoundType(value), true) => {
          field_offsets.push(field_name.to_string());

          let field_name_ident = format_ident!("{}", field_name);
          let type_name = format_ident!("{}", value);

          imp.push("let mut offset_vec = vec![];".to_string());

          match statements.resolve_decl_by_name(value) {
            DeclType::Struct(_) => {
              imp.push((quote! {
                for value in self.#field_name_ident {
                  offset_vec.push(#type_name::from(value));
                }

              }).to_string())
            }
            DeclType::Enum(_) => {
              imp.push((quote! {
                for value in self.#field_name_ident {
                  offset_vec.push(value);
                }
              }).to_string())
            }
            DeclType::Table(_) | DeclType::Union(_) => {
              imp.push((quote! {
                for value in self.#field_name_ident {
                  offset_vec.push(value.into_offset(builder));
                }
              }).to_string())
            }

            DeclType::Rpc(_) => {}
            DeclType::Null => {}
          };

          imp.push(format!("let {field_name}_offset = builder.create_vector(&offset_vec);"))
        }
        (ValueType::CompoundType(name), false) => {
          match statements.resolve_decl_by_name(name) {
            DeclType::Union(_) => {
              imp.push(format!("let {field_name}_offset = self.{field_name}.unwrap();"));
              unions.push(field_name.to_owned());
              field_offsets.push(field_name.to_string());
            }
            DeclType::Struct(_) => {
              imp.push(format!("let {field_name} = self.{field_name}.into();"));
              structs.push(field_name.to_owned());
            }
            DeclType::Enum(_) => {
              enums.push(field_name.to_owned());
            }
            DeclType::Table(_) => {
              imp.push(format!("let {field_name}_offset = self.{field_name}.into_offset(builder);"));
              field_offsets.push(field_name.to_string());
            }
            DeclType::Rpc(_) => {}
            DeclType::Null => {}
          };
        }
        (ValueType::String, false) => {
          field_offsets.push(field_name.to_string());
          imp.push(format!("let {field_name}_offset = builder.create_string(&self.{field_name});"))
        }
        (ValueType::String, true) => {
          field_offsets.push(field_name.to_string());
          let field_name = format_ident!("{}", field_name);
          let offset_name = format_ident!("{}_offset", field_name);
          let generated = quote! {
            let mut offset_vec = vec![];

            for value in self.#field_name {
              let str_offset = builder.create_string(&value);
              offset_vec.push(str_offset);
            }

            let #offset_name = builder.create_vector(&offset_vec);
          };


          imp.push(generated.to_string());
        }
        (_, true) => {
          field_offsets.push(field_name.to_string());
          imp.push(format!("let {field_name}_offset = builder.create_vector(&self.{field_name});"))
        }
        (_, false) => {
          straight_fields.push(field_name.to_owned());
        }
      }
    }

    let unions_as_args = unions.into_iter().map(|item| {
      format!("{item}_type: self.{item}_type,")
    }).collect::<Vec<String>>().join("\n");
    let offsets_as_args = field_offsets.into_iter().map(|field| {
      format!("{field}: Some({field}_offset),")
    }).collect::<Vec<String>>().join("\n");
    let straight_as_args = straight_fields.into_iter().map(|field| {
      format!("{field}: self.{field},")
    }).collect::<Vec<String>>().join("\n");

    let structs_as_args = structs.into_iter().map(|field| {
      format!("{field}: Some(&{field}),")
    }).collect::<Vec<String>>().join("\n");

    let enums_as_args = enums.into_iter().map(|field| {
      format!("{field}: self.{field},")
    }).collect::<Vec<String>>().join("\n");

    imp.push(format!("let args = &{}Args {{ \n {unions_as_args} {offsets_as_args} {straight_as_args} {structs_as_args} {enums_as_args} }};", self.name));
    imp.push(format!("{}::create(builder, args)", &self.name));

    imp.push("}\n}\n\n".to_owned());
    imp.join("\n")
  }
}

impl GenerateIntoByteVecImpl for TableDeclaration {
  fn generate_into_byte_vec_impl(&self, statements: &Statements) -> String {
    if &self.name == statements.root_type_name.as_ref().unwrap() {
      return "".to_owned();
    }

    let root_type_available_payload = statements.get_available_commands_union().unwrap();
    if !root_type_available_payload.items.contains(&self.name) {
      return "".to_owned();
    }


    let struct_name = format_ident!("{}Content", &self.name);
    let origin_name = format_ident!("{}", &self.name);
    let root_union_name = format_ident!("{}", &root_type_available_payload.name);
    let root_type_name = format_ident!("{}Content", &statements.root_type_name.clone().unwrap());


    let quote_attempt = quote! {
      impl From<#struct_name> for Vec<u8> {
        fn from(content: #struct_name) -> Self {
          let mut builder = FlatBufferBuilder::new();
          let id = content.id.to_owned();
          let payload = content.into_offset(&mut builder).as_union_value();
          let root_type = #root_type_name {
            id,
            payload_type: #root_union_name::#origin_name,
            payload: Some(payload),
          };

          let offset = root_type.into_offset(&mut builder);
          builder.finish(offset, None);

          Vec::from(builder.finished_data())
        }
      }
    };

    quote_attempt.to_string()
  }
}

impl ToRsStatement for TableDeclaration {
  fn to_rs_string(&self, statements: &Statements) -> String {
    let struct_name = format!("{}Content", self.name);

    let mut table_statement = vec![];
    table_statement.push(format!("pub struct {} {{", struct_name));
    let mut is_id_exist = false;

    for (field_name, field_type) in self.fields.iter() {
      match (&field_type.value_type, &field_type.is_array) {
        (ValueType::CompoundType(name), false) => {
          match statements.resolve_decl_by_name(name) {
            DeclType::Union(union) => {
              table_statement.push(format!("\tpub {}_type: {},", field_name, union.name));
              table_statement.push(format!("\tpub {}: Option<WIPOffset<UnionWIPOffset>>,", field_name))
            }
            DeclType::Enum(enum_decl) => {
              table_statement.push(format!("\tpub {}: {},", field_name, enum_decl.name));
            }
            DeclType::Table(_) | DeclType::Struct(_) => {
              table_statement.push(format!("\tpub {}: {},", field_name, field_type.to_rs_string(statements)))
            }
            DeclType::Rpc(_) => {}
            DeclType::Null => {}
          };
        }
        (ValueType::CompoundType(name), true) => {
          match statements.resolve_decl_by_name(name) {
            DeclType::Enum(enum_decl) => {
              table_statement.push(format!("\tpub {}: Vec<{}>,", field_name, enum_decl.name));
            }
            _ => {
              table_statement.push(format!("\tpub {}: {},", field_name, field_type.to_rs_string(statements)))
            }
          };
        }
        _ => { table_statement.push(format!("\tpub {}: {},", field_name, field_type.to_rs_string(statements))); }
      }


      if field_name == "id" {
        is_id_exist = true;
      }
    }

    if !is_id_exist && statements.get_available_commands_union().unwrap().items.contains(&self.name) {
      table_statement.push("\tpub id: String".to_owned());
    }
    table_statement.push("}\n\n".to_owned());
    table_statement.join("\n")
  }
}

impl ToRsStatement for RpcDeclaration {
  fn to_rs_string(&self, _: &Statements) -> String {
    let mut imp = vec![
      "pub trait RequestHandler {".to_owned()
    ];


    for method in self.methods.values() {
      let method_name = method.name.to_owned().to_case(Case::Snake);
      imp.push(format!("/** incoming must be {} */", method.input));
      imp.push(format!("fn {method_name}(incoming: Vec<u8>) -> JoinHandle<{}Content>;", method.output));
    };

    imp.push("}".to_owned());
    imp.join("\n")
  }
}

impl ToRsStatement for StructDeclaration {
  fn to_rs_string(&self, statements: &Statements) -> String {
    let mut imp = vec![];
    imp.push(format!("pub struct {}Content {{", self.name));

    for (name, type_def) in self.fields.iter() {
      imp.push(format!("\t pub {}: {},", name, type_def.to_rs_string(statements)));
    }

    imp.push("}".to_owned());
    imp.join("\n")
  }
}

impl GenerateIntoProtocolStructImpl for StructDeclaration {
  fn generate_into_protocol_struct_impl(&self) -> String {
    let name = &self.name;
    let mut imp = vec![
      format!("impl From<{name}Content> for {name} {{"),
      format!("fn from(struct_def: {name}Content) -> Self {{"),
      format!("{name}T {{"),
    ];

    for field_name in self.fields.keys() {
      imp.push(format!("{field_name}: struct_def.{field_name},"));
    };

    imp.push("}.pack()\n}\n}".to_owned());


    imp.join("\n")
  }
}
