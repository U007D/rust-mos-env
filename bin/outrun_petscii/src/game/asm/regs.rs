macro_rules! pop_regs {
    () => {
        "pla\ntay\npla\ntax\npla"
    };
}
pub(crate) use pop_regs;

macro_rules! push_regs {
    () => {
        "pha\ntxa\npha\ntya\npha"
    };
}
pub(crate) use push_regs;
