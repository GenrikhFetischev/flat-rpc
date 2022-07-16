use std::collections::{HashMap, HashSet};
use crate::parser::{Rule};
use pest::iterators::Pair;

#[derive(Debug, Clone)]
pub enum ValueType {
  Bool,
  Byte,
  Ubyte,
  Short,
  Ushort,
  Int,
  Uint,
  Float,
  Long,
  Ulong,
  Double,
  Int8,
  Uint8,
  Int16,
  Uint16,
  Int32,
  Uint32,
  Int64,
  Uint64,
  Float32,
  Float64,
  String,
  CompoundType(String),
}

impl From<&str> for ValueType {
  fn from(string: &str) -> Self {
    match string {
      "bool" => ValueType::Bool,
      "byte" => ValueType::Byte,
      "ubyte" => ValueType::Ubyte,
      "short" => ValueType::Short,
      "ushort" => ValueType::Ushort,
      "int" => ValueType::Int,
      "uint" => ValueType::Uint,
      "float" => ValueType::Float,
      "long" => ValueType::Long,
      "ulong" => ValueType::Ulong,
      "double" => ValueType::Double,
      "int8" => ValueType::Int8,
      "uint8" => ValueType::Uint8,
      "int16" => ValueType::Int16,
      "uint16" => ValueType::Uint16,
      "int32" => ValueType::Int32,
      "uint32" => ValueType::Uint32,
      "int64" => ValueType::Int64,
      "uint64" => ValueType::Uint64,
      "float32" => ValueType::Float32,
      "float64" => ValueType::Float64,
      "string" => ValueType::String,
      compound => ValueType::CompoundType(compound.to_owned())
    }
  }
}


#[derive(Debug, Clone)]
pub struct Type {
  pub value_type: ValueType,
  pub is_array: bool,
}

impl From<Pair<'_, Rule>> for Type {
  fn from(pair: Pair<'_, Rule>) -> Self {
    let typing = pair.into_inner().next().unwrap();

    match typing.as_rule() {
      Rule::array_type => {
        Type {
          value_type: typing.into_inner().as_str().into(),
          is_array: true,
        }
      }
      Rule::value_type => {
        Type {
          value_type: typing.as_str().into(),
          is_array: false,
        }
      }
      _ => unreachable!()
    }
  }
}


#[derive(Debug)]
pub struct TableDeclaration {
  pub name: String,
  pub fields: HashMap<String, Type>,
  pub fields_order: Vec<String>,
}

impl From<Pair<'_, Rule>> for TableDeclaration {
  fn from(pair: Pair<'_, Rule>) -> Self {
    let mut table_decl = pair.into_inner();

    let identifier = table_decl.next().unwrap();
    let name = identifier.as_span().as_str().to_owned();
    let mut fields_order = vec![];

    let fields = table_decl.into_iter().map(|field| {
      let mut field = field.into_inner();
      let field_name = field.next().unwrap().as_span().as_str();
      let field_type = Type::from(field.next().unwrap());
      (field_name, field_type)
    }).fold(HashMap::new(), |mut acc: HashMap<String, Type>, (key, value)| {
      fields_order.push(key.to_owned());
      acc.insert(key.to_owned(), value);
      acc
    });

    TableDeclaration {
      name,
      fields,
      fields_order,
    }
  }
}


#[derive(Debug)]
pub struct EnumDeclaration {
  pub name: String,
  pub type_def: Type,
  pub items: HashSet<String>,
}

impl From<Pair<'_, Rule>> for EnumDeclaration {
  fn from(pair: Pair<'_, Rule>) -> Self {
    let mut enum_ast = pair.into_inner();
    let identifier = enum_ast.next().unwrap();
    let name = identifier.as_str().to_owned();

    let type_def = enum_ast.next().unwrap();

    let type_def = Type::from(type_def);

    let items = enum_ast.into_iter().map(|field| {
      let mut field = field.into_inner();
      field.next().unwrap().as_str().to_owned()
    }).fold(HashSet::new(), |mut acc: HashSet<String>, item| {
      acc.insert(item);
      acc
    });


    EnumDeclaration {
      name,
      items,
      type_def,
    }
  }
}


#[derive(Debug)]
pub struct UnionDeclaration {
  pub name: String,
  pub items: HashSet<String>,
}

impl From<Pair<'_, Rule>> for UnionDeclaration {
  fn from(pair: Pair<'_, Rule>) -> Self {
    let mut union_ast = pair.into_inner();
    let identifier = union_ast.next().unwrap();
    let name = identifier.as_str().to_owned();

    let items = union_ast.into_iter().map(|field| {
      let mut field = field.into_inner();
      field.next().unwrap().as_str().to_owned()
    }).fold(HashSet::new(), |mut acc: HashSet<String>, item| {
      acc.insert(item);
      acc
    });


    UnionDeclaration {
      name,
      items,
    }
  }
}

#[derive(Debug)]
pub struct RpcMethod {
  pub name: String,
  pub input: String,
  pub output: String,
}

#[derive(Debug)]
pub struct RpcDeclaration {
  pub name: String,
  pub methods: HashMap<String, RpcMethod>,
}

impl From<Pair<'_, Rule>> for RpcDeclaration {
  fn from(rule: Pair<'_, Rule>) -> Self {
    let mut methods = HashMap::new();
    let mut rpc_ast = rule.into_inner();
    let service_name = rpc_ast.next().unwrap().as_str().to_owned();


    for method in rpc_ast {
      let mut method = method.into_inner();
      let method_name = method.next().unwrap().as_str().to_owned();
      let input = method.next().unwrap().as_str().to_owned();
      let output = method.next().unwrap().as_str().to_owned();

      methods.insert(method_name.to_owned(), RpcMethod {
        name: method_name,
        input,
        output,
      });
    };

    RpcDeclaration {
      name: service_name,
      methods,
    }
  }
}


#[derive(Debug)]
pub struct StructDeclaration {
  pub name: String,
  pub fields: HashMap<String, Type>,
  pub fields_order: Vec<String>,
}

impl From<Pair<'_, Rule>> for StructDeclaration {
  fn from(pair: Pair<'_, Rule>) -> Self {
    let mut struct_decl = pair.into_inner();

    let identifier = struct_decl.next().unwrap();
    let name = identifier.as_span().as_str().to_owned();
    let mut fields_order = vec![];

    let fields = struct_decl.into_iter().map(|field| {
      let mut field = field.into_inner();
      let field_name = field.next().unwrap().as_span().as_str();
      let field_type = Type::from(field.next().unwrap());
      (field_name, field_type)
    }).fold(HashMap::new(), |mut acc: HashMap<String, Type>, (key, value)| {
      fields_order.push(key.to_owned());
      acc.insert(key.to_owned(), value);
      acc
    });

    StructDeclaration {
      name,
      fields,
      fields_order,
    }
  }
}
