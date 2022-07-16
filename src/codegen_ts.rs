use convert_case::{Case, Casing};

use crate::parser::{DeclType, Statements};
use crate::ir::{TableDeclaration, RpcDeclaration, Type, UnionDeclaration, ValueType, StructDeclaration};


pub fn generate_ts_client_side_code(statements: &Statements) -> String {
  let mut generated = vec![generate_header(statements)];

  for table in statements.table_declaration.values() {
    generated.push(table.to_ts_string(statements));
    generated.push(table.generate_into_function_statement(statements));
  };

  for struct_decl in statements.struct_declaration.values() {
    generated.push(struct_decl.to_ts_string(statements));
    generated.push(struct_decl.generate_into_function_statement(statements));
  }

  for union in statements.unions.values() {
    generated.push(union.to_ts_string(statements));
  }

  for rpc in statements.rpc_declarations.values() {
    generated.push(rpc.to_ts_string(statements));
  }

  generated.join("\n")
}


pub fn generate_header(statements: &Statements) -> String {
  let mut imports = vec![];

  for table in statements.table_declaration.values() {
    imports.push(table.name.to_owned());
    imports.push(format!("{}T", table.name.to_owned()));
  }


  for struct_decl in statements.struct_declaration.values() {
    imports.push(struct_decl.name.to_owned());
    imports.push(format!("{}T", struct_decl.name.to_owned()));
  }

  for name in statements.unions.keys() {
    imports.push(name.to_owned().to_string());
  }

  for name in statements.enum_declarations.keys() {
    imports.push(name.to_owned().to_string());
  }

  format!(r#"import * as fb from "flatbuffers";
const {{Builder}} = fb;
import {{ {}  }} from "./schema_generated";
export type Transport = {{
	sendMessage: (msg: Uint8Array, id: string) => Promise<RootTable>
}}

"#, imports.join(", "))
}


pub trait ToTsStatement {
  fn to_ts_string(&self, statements: &Statements) -> String;
}

pub trait GenerateIntoFunctionStatement {
  fn generate_into_function_statement(&self, statements: &Statements) -> String;
}


impl ToTsStatement for ValueType {
  fn to_ts_string(&self, _: &Statements) -> String {
    match self {
      ValueType::Bool => "boolean".to_owned(),
      ValueType::String => "string".to_owned(),

      ValueType::Byte |
      ValueType::Ubyte |
      ValueType::Short |
      ValueType::Ushort |
      ValueType::Int |
      ValueType::Uint |
      ValueType::Float |
      ValueType::Int8 |
      ValueType::Uint8 |
      ValueType::Int16 |
      ValueType::Uint16 |
      ValueType::Int32 |
      ValueType::Uint32 |
      ValueType::Float32 => "number".to_owned(),

      ValueType::Int64 |
      ValueType::Uint64 |
      ValueType::Long |
      ValueType::Ulong |
      ValueType::Double |
      ValueType::Float64 => "bigint".to_owned(),

      ValueType::CompoundType(compound) => format!("{compound}Content")
    }
  }
}

impl ToTsStatement for Type {
  fn to_ts_string(&self, statements: &Statements) -> String {
    if self.is_array {
      format!("Array<{}>", self.value_type.to_ts_string(statements))
    } else {
      self.value_type.to_ts_string(statements)
    }
  }
}

impl ToTsStatement for TableDeclaration {
  fn to_ts_string(&self, statements: &Statements) -> String {
    let struct_name = format!("{}Content", self.name);

    let mut type_definition = vec![];
    type_definition.push(format!("export type {} = {{", struct_name));
    let mut is_id_exist = false;

    for (field_name, field_type) in self.fields.iter() {
      match (&field_type.value_type, &field_type.is_array) {
        (ValueType::CompoundType(name), false) => {
          if let Some(union) = statements.unions.get(name) {
            type_definition.push(format!("\t{}Type: {},", field_name, union.name));
            type_definition.push(format!("\t{}: {}Content,", field_name, union.name))
          } else if let Some(enum_decl) = statements.enum_declarations.get(name) {
            type_definition.push(format!("\t{}: {},", field_name, enum_decl.name))
          } else {
            type_definition.push(format!("\t{}: {},", field_name, field_type.to_ts_string(statements)))
          }
        }
        (ValueType::CompoundType(name), true) => {
          if let Some(enum_decl) = statements.enum_declarations.get(name) {
            type_definition.push(format!("\t{}: Array<{}>,", field_name, enum_decl.name))
          } else {
            type_definition.push(format!("\t{}: {},", field_name, field_type.to_ts_string(statements)))
          }
        }
        _ => {
          if field_name == "id" {
            is_id_exist = true;
            type_definition.push(format!("\t{}?: {},", field_name, field_type.to_ts_string(statements)));
          } else {
            type_definition.push(format!("\t{}: {},", field_name, field_type.to_ts_string(statements)));
          }
        }
      }
    }
    if !is_id_exist {
      type_definition.push("\tid?: string".to_owned());
    }
    type_definition.push("}\n\n".to_owned());
    type_definition.join("\n")
  }
}

impl GenerateIntoFunctionStatement for TableDeclaration {
  fn generate_into_function_statement(&self, statements: &Statements) -> String {
    let origin_name = self.name.as_str();
    let name = format!("{}Content", self.name);
    let function_name_prefix = name.to_case(Case::Camel);
    let mut imp = vec![
      format!("const {function_name_prefix}IntoProtocolClass = (content: {name}): {origin_name}T => {{"),
    ];

    let mut fields_as_args = vec![];


    for field_name in &self.fields_order {
      let field = self.fields.get(field_name).unwrap();
      match (&field.value_type, field.is_array) {
        (ValueType::CompoundType(name), false) => {
          let generated_name = format!("{name}Content");

          match statements.resolve_decl_by_name(name) {
            DeclType::Table(_) | DeclType::Struct(_) => {
              let function_name_prefix = generated_name.to_case(Case::Camel);
              fields_as_args.push(format!("{function_name_prefix}IntoProtocolClass(content.{field_name})"))
            }
            DeclType::Enum(_) => {
              fields_as_args.push(format!("content.{field_name}"))
            }
            _ => {}
          };
        }
        (ValueType::CompoundType(name), true) => {
          match statements.resolve_decl_by_name(name) {
            DeclType::Table(_) | DeclType::Struct(_) => {
              let generated_name = format!("{name}Content");
              let function_name_prefix = generated_name.to_case(Case::Camel);
              fields_as_args.push(format!("content.{field_name}.map({function_name_prefix}IntoProtocolClass)"))
            }
            DeclType::Enum(_) => {
              fields_as_args.push(format!("content.{field_name}"))
            }
            _ => {}
          };
        }
        _ => {
          fields_as_args.push(format!("content.{field_name}"))
        }
      }
    };

    let fields_as_args = fields_as_args.join(", ");

    imp.push(format!("return new {origin_name}T({fields_as_args})"));

    imp.push("}".to_owned());
    imp.join("\n")
  }
}


impl ToTsStatement for UnionDeclaration {
  fn to_ts_string(&self, _: &Statements) -> String {
    let enum_name = self.name.to_owned();

    let mut enum_definition = vec![];
    enum_definition.push(format!("export type {enum_name}Content = "));


    for variant in self.items.iter() {
      enum_definition.push(format!("\t | {variant}"));
    }

    enum_definition.push("\n\n".to_owned());
    enum_definition.join("\n")
  }
}


impl ToTsStatement for RpcDeclaration {
  fn to_ts_string(&self, statements: &Statements) -> String {
    let mut methods = vec![];
    let mut imp = vec![];


    for method in self.methods.values() {
      let method_name = method.name.to_owned().to_case(Case::Camel);
      let input = &method.input;
      let output = &method.output;

      let root_union_name = statements.get_available_commands_union().unwrap().name.as_str();
      let root_table_name = statements.root_type_name.as_ref().unwrap();

      let input_protocol_interface = statements.table_declaration.get(input).unwrap();

      methods.push(method_name.to_owned());
      imp.push(format!("export const {method_name} = async (transport: Transport, content: {input}Content): Promise<{output}Content> => {{", ));
      imp.push("const builder = new Builder();".to_owned());

      let into_function_prefix = input_protocol_interface.name.as_str().to_case(Case::Camel);

      imp.push(format!("let protocolPackage = {into_function_prefix}ContentIntoProtocolClass(content);"));
      imp.push("let payloadOffset = protocolPackage.pack(builder);".to_owned());
      imp.push("let id = content.id ?? self.crypto.randomUUID();".to_owned());
      imp.push("const idOffset = builder.createString(id);".to_owned());
      imp.push(format!("const root = {root_table_name}.create{root_table_name}(builder, idOffset, {root_union_name}.{input}, payloadOffset);"));

      imp.push("builder.finish(root)".to_owned());
      imp.push(format!("const response: {root_table_name} = await transport.sendMessage(builder.asUint8Array(), id);"));
      imp.push(format!("const responseData = new {output}();"));
      imp.push("response.payload(responseData);".to_owned());
      imp.push(format!(r#"return {{
        ...responseData.unpack(),
        id
      }} as unknown as {output}Content"#).to_owned());

      imp.push("}".to_owned());
    };


    imp.push("export const createApiObject = (transport: Transport) => {".to_owned());
    imp.push("return {".to_owned());

    imp.push(methods.into_iter().map(|method| format!("{method}: {method}.bind(null, transport),")).collect::<Vec<String>>().join("\n"));

    imp.push("}".to_owned());
    imp.push("}".to_owned());
    imp.join("\n")
  }
}


impl ToTsStatement for StructDeclaration {
  fn to_ts_string(&self, statements: &Statements) -> String {
    let name = &self.name;
    let mut imp = vec![
      format!("export type {name}Content = {{"),
    ];
    for (name, type_def) in self.fields.iter() {
      imp.push(format!("{name}: {};", type_def.value_type.to_ts_string(statements)));
    }
    imp.push("};".to_owned());


    imp.join("\n")
  }
}

impl GenerateIntoFunctionStatement for StructDeclaration {
  fn generate_into_function_statement(&self, _: &Statements) -> String {
    let name = &self.name;
    let function_name_prefix = name.to_case(Case::Camel);
    let mut imp = vec![
      format!("export const {function_name_prefix}ContentIntoProtocolClass = (content: {name}Content): {name}T => {{")
    ];

    let fields_as_args = self.fields_order.iter().map(|field_name| {
      format!("content.{field_name},")
    }).collect::<Vec<String>>().join("\n");

    imp.push(format!("return new {name}T({fields_as_args})"));
    imp.push("};".to_owned());


    imp.join("\n")
  }
}
