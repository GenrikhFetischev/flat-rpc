use crate::parser::Statements;


/** Check if all RPC tables are in root table payload union */
fn check_root_union(statements: &Statements) -> Option<String> {
  let rpc_types = statements.rpc_declarations.values().flat_map(|rpc_decl| {
    rpc_decl.methods.values().flat_map(|method| {
      vec![method.input.to_owned(), method.output.to_owned()]
    }).collect::<Vec<String>>()
  }).collect::<Vec<String>>();


  let root_union = statements.get_available_commands_union().unwrap();

  let missed_tables = rpc_types.iter().filter_map(|item| {
    if root_union.items.contains(item) {
      None
    } else {
      Some(item.to_owned())
    }
  })
    .collect::<Vec<String>>();


  if missed_tables.is_empty() {
    None
  } else {
    Some(format!("Following tables included into RPC statemtns, but not presented in root table payload union: {}", missed_tables.join(", ")))
  }
}


pub fn type_check(statements: &Statements) -> Option<String> {
  let mut errors = vec![];
  if statements.root_type_name.is_none() {
    errors.push("There is no root type in the schema".to_owned())
  };

  if statements.get_available_commands_union().is_none() {
    errors.push("There is no root type in the schema".to_owned())
  } else if let Some(errs) = check_root_union(statements) {
    errors.push(errs);
  };

  if errors.is_empty() {
    None
  } else {
    Some(errors.join("\n"))
  }
}
