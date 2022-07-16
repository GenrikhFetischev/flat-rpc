use std::collections::HashMap;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

use crate::ir::{TableDeclaration, RpcDeclaration, UnionDeclaration, ValueType, StructDeclaration, EnumDeclaration};

#[derive(Parser)]
#[grammar = "./grammar.pest"]
pub struct FbsParser {}


#[derive(Default, Debug)]
pub struct Statements {
  pub root_type_name: Option<String>,
  pub table_declaration: HashMap<String, TableDeclaration>,
  pub struct_declaration: HashMap<String, StructDeclaration>,
  pub unions: HashMap<String, UnionDeclaration>,
  pub rpc_declarations: HashMap<String, RpcDeclaration>,
  pub enum_declarations: HashMap<String, EnumDeclaration>,
}


pub enum DeclType<'a> {
  Table(&'a TableDeclaration),
  Struct(&'a StructDeclaration),
  Enum(&'a EnumDeclaration),
  Union(&'a UnionDeclaration),
  Rpc(&'a RpcDeclaration),
  Null,
}


impl Statements {
  pub fn get_available_commands_union(&self) -> Option<&UnionDeclaration> {
    let root_type_name = self.root_type_name.clone().expect("there is no root type name");

    let root_type = self.table_declaration.get(&root_type_name).expect("there is no root type");
    let payload = root_type.fields.get("payload").expect("there is no payload in root type");

    let union_name = match &payload.value_type {
      ValueType::CompoundType(union_name) => union_name,
      _ => panic!("union root")
    };
    self.unions.get(union_name)
  }

  pub fn resolve_decl_by_name(&self, name: &str) -> DeclType {
    if let Some(table_decl) = self.table_declaration.get(name) {
      return DeclType::Table(table_decl);
    } else if let Some(struct_decl) = self.struct_declaration.get(name) {
      return DeclType::Struct(struct_decl);
    } else if let Some(enum_decl) = self.enum_declarations.get(name) {
      return DeclType::Enum(enum_decl);
    } else if let Some(union_decl) = self.unions.get(name) {
      return DeclType::Union(union_decl);
    } else if let Some(rpc_decl) = self.rpc_declarations.get(name) {
      return DeclType::Rpc(rpc_decl);
    }

    DeclType::Null
  }
}


pub fn parse_fbs_schema(schema_string: &str) -> Statements {
  let parsed = FbsParser::parse(Rule::schema, schema_string);
  let mut file = match parsed {
    Ok(parse_result) =>
      parse_result,
    Err(e) => {
      panic!("Error while parsing file {:?}", e)
    }
  };


  let mut statements = Statements::default();


  let file = file.next().unwrap();
  for statement in file.into_inner() {
    match statement.as_rule() {
      Rule::table_decl => {
        let table_decl = TableDeclaration::from(statement);
        statements.table_declaration.insert(table_decl.name.to_owned(), table_decl);
      }
      Rule::struct_decl => {
        let struct_decl = StructDeclaration::from(statement);
        statements.struct_declaration.insert(struct_decl.name.to_owned(), struct_decl);
      }
      Rule::root_decl => {
        let statement: Pair<'_, Rule> = statement;
        let mut root_type = statement.into_inner();
        let name = root_type.next().unwrap().as_str().to_owned();
        statements.root_type_name = Some(name.to_owned());
      }
      Rule::union_decl => {
        let union = UnionDeclaration::from(statement);
        statements.unions.insert(union.name.to_owned(), union);
      }
      Rule::enum_decl => {
        let enum_decl = EnumDeclaration::from(statement);
        statements.enum_declarations.insert(enum_decl.name.to_owned(), enum_decl);
      }
      Rule::rpc_decl => {
        let rpc_decl = RpcDeclaration::from(statement);
        statements.rpc_declarations.insert("".to_owned(), rpc_decl);
      }
      _ => {}
    }
  }

  statements
}











