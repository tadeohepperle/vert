use super::{ModuleId, UntypedHandle};

pub trait Dependencies {
    fn type_ids() -> Vec<ModuleId>;
    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self;
}

impl Dependencies for () {
    fn type_ids() -> Vec<ModuleId> {
        vec![]
    }

    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self {
        assert_eq!(ptrs.len(), 0);
        ()
    }
}

impl<A: Dependencies, B: Dependencies> Dependencies for (A, B) {
    fn type_ids() -> Vec<ModuleId> {
        let mut e = vec![];
        e.extend(A::type_ids());
        e.extend(B::type_ids());
        e
    }

    fn from_untyped_handles(ptrs: &[UntypedHandle]) -> Self {
        let a: A;
        let b: B;

        let mut offset = -(A::type_ids().len() as i32);

        {
            let a_len = A::type_ids().len() as i32;
            offset += a_len;
            a = A::from_untyped_handles(&ptrs[offset as usize..(offset + a_len) as usize]);
        }

        {
            let b_len = B::type_ids().len() as i32;
            offset += b_len;
            b = B::from_untyped_handles(&ptrs[offset as usize..(offset + b_len) as usize]);
        }

        (a, b)
    }
}

macro_rules! implement_dependencies_t {
    ( $(),*) => {};
}
