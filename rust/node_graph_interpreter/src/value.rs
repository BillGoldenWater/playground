use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Debug, Clone, Default)]
pub enum Value {
    #[default]
    Uninit,
    None,

    Bool(bool),
    Int(i64),

    String(Arc<str>),
    List(Rc<RefCell<Vec<Value>>>),

    // LoopId(usize),
    LocalVariable(usize),
}

impl Value {
    pub fn as_bool(&self) -> bool {
        let Self::Bool(v) = self else {
            panic!("expect bool, actual: {self:?}");
        };

        *v
    }

    pub fn as_int(&self) -> i64 {
        let Self::Int(v) = self else {
            panic!("expect int, actual: {self:?}");
        };

        *v
    }

    pub fn as_str(&self) -> &str {
        let Self::String(v) = self else {
            panic!("expect string, actual: {self:?}");
        };

        v
    }

    pub fn as_list(&self) -> &RefCell<Vec<Value>> {
        let Self::List(v) = self else {
            panic!("expect list, actual: {self:?}");
        };

        v
    }

    pub fn as_local_variable(&self) -> usize {
        let Self::LocalVariable(v) = self else {
            panic!("expect local variable, actual: {self:?}");
        };

        *v
    }

    // pub fn as_loop_id(&self) -> usize {
    //     let Self::LoopId(v) = self else {
    //         panic!("expect loop id, actual: {self:?}");
    //     };
    //
    //     *v
    // }

    /// Returns `true` if the value is [`Uninit`].
    ///
    /// [`Uninit`]: Value::Uninit
    #[must_use]
    pub fn is_uninit(&self) -> bool {
        matches!(self, Self::Uninit)
    }
}
