
#[derive(Clone)]
pub struct Program {
	pub variables: Vec<String>,
	pub instructions: Vec<Instruction>,
}

#[derive(Clone)]
pub enum Instruction {
	Assign {to: String, expr: Expr},
	If {
		condition: Expr,
		body: Vec<Instruction>,
	},
	Loop {
		times: Value,
		body: Vec<Instruction>,
	}
}

#[derive(Clone)]
pub struct Expr {
	pub left: Value,
	pub right: Value,
	pub op: Operation,
}

#[derive(Clone, Copy)]
pub enum Operation {
	Plus,
	Minus,
	Times,
	Divided,
	Modulo,
	Equal,
	NotEqual,
}

#[derive(Clone)]
pub enum Value {
	Variable(String),
	Constant(i64),
}