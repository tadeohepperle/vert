pub trait CollectableTrait {
    type TraitPointer: ?Sized + 'static;
}

impl CollectableTrait for CollectDescribeMe {
    type TraitPointer = dyn DescribeMe;
}

pub struct CollectDescribeMe;
pub trait DescribeMe {
    fn print(&self) -> String;
}
