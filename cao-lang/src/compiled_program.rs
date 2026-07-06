use std::{io::Write, mem::transmute, str::FromStr};

use crate::{
    collections::{
        handle_table::{Handle, HandleTable},
        hash_map::CaoHashMap,
    },
    compiler::{CardIndex, NameSpace},
    instruction::Instruction,
    vm::instr_execution::decode_value,
    VarName,
};
use crate::{version, VariableId};

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Labels(pub HandleTable<Label>);

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Variables {
    pub ids: HandleTable<VariableId>,
    pub names: HandleTable<VarName>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Label {
    /// Position of this card in the bytecode of the program
    pub pos: u32,
}

impl Label {
    pub fn new(pos: u32) -> Self {
        Self { pos }
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trace {
    pub namespace: NameSpace,
    pub index: CardIndex,
}

impl std::fmt::Display for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for ns in self.namespace.iter() {
            write!(f, "{ns}.")?;
        }
        write!(f, "{}", self.index)
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CaoCompiledProgram {
    /// Instructions
    pub bytecode: Vec<u8>,
    /// Data used by instuctions with variable length inputs
    pub data: Vec<u8>,
    pub labels: Labels,
    pub variables: Variables,
    pub cao_lang_version: String,
    pub trace: CaoHashMap<u32, Trace>,
}

fn write_instruction_name(instr: Instruction, mut writer: impl Write) -> std::io::Result<()> {
    let name = match instr {
        Instruction::Add => "Add",
        Instruction::Sub => "Sub",
        Instruction::Mul => "Mul",
        Instruction::Div => "Div",
        Instruction::CallNative => "CallNative",
        Instruction::ScalarInt => "ScalarInt",
        Instruction::ScalarFloat => "ScalarFloat",
        Instruction::ScalarNil => "ScalarNil",
        Instruction::StringLiteral => "StringLiteral",
        Instruction::CopyLast => "CopyLast",
        Instruction::Exit => "Exit",
        Instruction::CallFunction => "CallFunction",
        Instruction::Equals => "Equals",
        Instruction::NotEquals => "NotEquals",
        Instruction::Less => "Less",
        Instruction::LessOrEq => "LessOrEq",
        Instruction::Pop => "Pop",
        Instruction::SetGlobalVar => "SetGlobalVar",
        Instruction::ReadGlobalVar => "ReadGlobalVar",
        Instruction::SetLocalVar => "SetLocalVar",
        Instruction::ReadLocalVar => "ReadLocalVar",
        Instruction::ClearStack => "ClearStack",
        Instruction::Return => "Return",
        Instruction::SwapLast => "SwapLast",
        Instruction::And => "And",
        Instruction::Or => "Or",
        Instruction::Xor => "Xor",
        Instruction::Not => "Not",
        Instruction::GotoIfTrue => "GotoIfTrue",
        Instruction::GotoIfFalse => "GotoIfFalse",
        Instruction::Goto => "Goto",
        Instruction::InitTable => "InitTable",
        Instruction::GetProperty => "GetProperty",
        Instruction::SetProperty => "SetProperty",
        Instruction::Len => "Len",
        Instruction::BeginForEach => "BeginForEach",
        Instruction::ForEach => "ForEach",
        Instruction::FunctionPointer => "FunctionPointer",
        Instruction::NativeFunctionPointer => "NativeFunctionPointer",
        Instruction::NthRow => "NthRow",
        Instruction::AppendTable => "AppendTable",
        Instruction::PopTable => "PopTable",
        Instruction::Closure => "Closure",
        Instruction::SetUpvalue => "SetUpvalue",
        Instruction::ReadUpvalue => "ReadUpvalue",
        Instruction::RegisterUpvalue => "RegisterUpvalue",
        Instruction::CloseUpvalue => "CloseUpvalue",
    };
    write!(writer, "{}", name)
}

fn write_instruction_args<T: bytemuck::Pod + std::fmt::Debug>(
    mut writer: impl Write,
    names: &[&str],
    mut instruction_ptr: usize,
    bytecode: &[u8],
) -> std::io::Result<()> {
    instruction_ptr += 1;
    for name in names {
        let value: T = unsafe { decode_value(bytecode, &mut instruction_ptr) };
        write!(writer, "\t{}={:?}", name, value)?;
    }
    Ok(())
}

impl CaoCompiledProgram {
    pub fn variable_id(&self, name: &str) -> Option<VariableId> {
        self.variables
            .ids
            .get(Handle::from_str(name).unwrap())
            .copied()
    }

    pub fn print_disassembly(&self) {
        let mut out = std::io::BufWriter::new(std::io::stdout());
        self.disassemble_writer(&mut out).unwrap();
    }

    pub fn disassemble_string(&self) -> String {
        let mut out = Vec::new();
        self.disassemble_writer(&mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    pub fn disassemble_writer(&self, mut writer: impl std::io::Write) -> std::io::Result<()> {
        let mut i = 0;
        while i < self.bytecode.len() {
            let instr: u8 = self.bytecode[i];
            let instr: Instruction = unsafe { transmute(instr) };
            write!(writer, "{i}\t")?;
            // TODO: also print the arguments of the instructions
            write_instruction_name(instr, &mut writer)?;
            match instr {
                Instruction::CallFunction
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::ScalarNil
                | Instruction::CopyLast
                | Instruction::Exit
                | Instruction::Equals
                | Instruction::NotEquals
                | Instruction::Less
                | Instruction::LessOrEq
                | Instruction::Pop
                | Instruction::ClearStack
                | Instruction::Return
                | Instruction::SwapLast
                | Instruction::And
                | Instruction::Or
                | Instruction::Xor
                | Instruction::Not
                | Instruction::InitTable
                | Instruction::GetProperty
                | Instruction::SetProperty
                | Instruction::Len
                | Instruction::NthRow
                | Instruction::AppendTable
                | Instruction::PopTable
                | Instruction::CloseUpvalue
                | Instruction::Add => {}
                Instruction::GotoIfTrue | Instruction::GotoIfFalse | Instruction::Goto => {
                    write_instruction_args::<i32>(&mut writer, &["pos"], i, &self.bytecode)?;
                }
                Instruction::CallNative
                | Instruction::NativeFunctionPointer
                | Instruction::StringLiteral => {
                    // TODO: resolve the handle in the data section
                    write_instruction_args::<Handle>(&mut writer, &["handle"], i, &self.bytecode)?;
                }
                Instruction::BeginForEach | Instruction::ForEach => {
                    write_instruction_args::<u32>(
                        &mut writer,
                        &["loop_var", "loop_item", "i_index", "k_index", "v_index"],
                        i,
                        &self.bytecode,
                    )?;
                }
                Instruction::FunctionPointer | Instruction::Closure => {
                    // TODO: resolve the handle in the data section
                    write_instruction_args::<Handle>(&mut writer, &["handle"], i, &self.bytecode)?;
                    write_instruction_args::<u32>(&mut writer, &["arity"], i, &self.bytecode)?;
                }
                Instruction::SetUpvalue | Instruction::ReadUpvalue => {
                    write_instruction_args::<u32>(&mut writer, &["index"], i, &self.bytecode)?;
                }
                Instruction::RegisterUpvalue => {
                    write_instruction_args::<u8>(
                        &mut writer,
                        &["index", "is_local"],
                        i,
                        &self.bytecode,
                    )?;
                }
                Instruction::ScalarInt => {
                    write_instruction_args::<i64>(&mut writer, &["value"], i, &self.bytecode)?;
                }
                Instruction::ScalarFloat => {
                    write_instruction_args::<f64>(&mut writer, &["value"], i, &self.bytecode)?;
                }
                Instruction::SetGlobalVar | Instruction::ReadGlobalVar => {
                    write_instruction_args::<VariableId>(&mut writer, &["id"], i, &self.bytecode)?;
                }
                Instruction::SetLocalVar | Instruction::ReadLocalVar => {
                    write_instruction_args::<VariableId>(
                        &mut writer,
                        &["index"],
                        i,
                        &self.bytecode,
                    )?;
                }
            }
            writeln!(writer)?;
            i += instr.span();
        }
        Ok(())
    }
}

impl Default for CaoCompiledProgram {
    fn default() -> Self {
        Self {
            bytecode: Default::default(),
            data: Default::default(),
            labels: Default::default(),
            variables: Default::default(),
            cao_lang_version: version::VERSION_STR.to_string(),
            trace: Default::default(),
        }
    }
}
