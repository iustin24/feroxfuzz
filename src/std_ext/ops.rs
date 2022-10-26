#![allow(clippy::use_self)] // clippy false-positive on Action, doesn't want to apply directly to the enums that derive Serialize
use crate::actions::{Action, FlowControl};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet, LinkedList, VecDeque};
use std::ops::{BitAnd, BitOr};
use std::rc::Rc;
use std::sync::Arc;

use tracing::instrument;

/// simple trait to add a `.len()` equivalent to implementors
#[allow(clippy::len_without_is_empty)]
pub trait Len {
    /// get the length of the implementing object
    #[must_use]
    fn len(&self) -> usize;
}

macro_rules! impl_single_len {
    ($implementor:ty) => {
        impl Len for $implementor {
            fn len(&self) -> usize {
                <Self>::len(&self)
            }
        }

        impl Len for &$implementor {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }

        impl Len for Rc<$implementor> {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }

        impl Len for Arc<$implementor> {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }
    };
}

macro_rules! impl_double_len {
    ($implementor:ty, $gen:tt) => {
        impl<$gen> Len for $implementor {
            fn len(&self) -> usize {
                <Self>::len(&self)
            }
        }

        impl<$gen> Len for &$implementor {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }

        impl<$gen> Len for Rc<$implementor> {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }

        impl<$gen> Len for Arc<$implementor> {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }
    };
}

macro_rules! impl_triple_len {
    ($implementor:ty, $gen:tt, $gen2:tt) => {
        impl<$gen, $gen2> Len for $implementor {
            fn len(&self) -> usize {
                <Self>::len(&self)
            }
        }

        impl<$gen, $gen2> Len for &$implementor {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }

        impl<$gen, $gen2> Len for Rc<$implementor> {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }

        impl<$gen, $gen2> Len for Arc<$implementor> {
            fn len(&self) -> usize {
                <$implementor>::len(&self)
            }
        }
    };
}

impl_single_len!(String);
impl_single_len!(str);

impl_double_len!(BinaryHeap<T>, T);
impl_double_len!(LinkedList<T>, T);
impl_double_len!(VecDeque<T>, T);
impl_double_len!(Vec<T>, T);
impl_double_len!(BTreeSet<T>, T);

impl_triple_len!(BTreeMap<K, V>, K, V);
impl_triple_len!(HashSet<T, S>, T, S);

impl<T> Len for &[T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

impl Len for usize {
    fn len(&self) -> usize {
        self.to_ne_bytes().len()
    }
}

/// represents the logical joining of two [`Action`]s
///
/// [`Action::Keep`] and [`Action::Discard`] are analogous to true and false
/// and bitwise operations work on them the same way they would on true and false.
///
/// # Truth tables
///
/// ## `LogicOperation::And` operation on an [`Action`]
///
/// |-------------------------------------|-------------------------------------|-------------------------------------|
/// | A                                   | B                                   | A & B                               |
/// |-------------------------------------|-------------------------------------|-------------------------------------|
/// | `Keep`                              | `Keep`                              | `Keep`                              |
/// | `Keep`                              | `Discard`                           | `Discard`                           |
/// | `Keep`                              | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    |
/// | `Keep`                              | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` |
/// | `Discard`                           | `Keep`                              | `Discard`                           |
/// | `Discard`                           | `Discard`                           | `Discard`                           |
/// | `Discard`                           | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Discard)` |
/// | `Discard`                           | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Keep)`    | `Keep`                              | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Keep)`    | `Discard`                           | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Discard)` | `Keep`                              | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Discard)` | `Discard`                           | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` |
/// |-------------------------------------|-------------------------------------|-------------------------------------|
///
/// ## `LogicOperation::Or` operation on an [`Action`]
///
/// |-------------------------------------|-------------------------------------|-------------------------------------|
/// | A                                   | B                                   | A | B                               |
/// |-------------------------------------|-------------------------------------|-------------------------------------|
/// | `Keep`                              | `Keep`                              | `Keep`                              |
/// | `Keep`                              | `Discard`                           | `Keep`                              |
/// | `Keep`                              | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    |
/// | `Keep`                              | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Keep)`    |
/// | `Discard`                           | `Keep`                              | `Keep`                              |
/// | `Discard`                           | `Discard`                           | `Discard`                           |
/// | `Discard`                           | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    |
/// | `Discard`                           | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Keep)`    | `Keep`                              | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Keep)`    | `Discard`                           | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Discard)` | `Keep`                              | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Discard)` | `Discard`                           | `AddToCorpus(FlowControl::Discard)` |
/// | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Keep)`    | `AddToCorpus(FlowControl::Keep)`    |
/// | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` | `AddToCorpus(FlowControl::Discard)` |
/// |-------------------------------------|-------------------------------------|-------------------------------------|
///
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum LogicOperation {
    /// logical AND, implemented as a bitwise operation
    And,

    /// logical OR, implemented as a bitwise operation
    #[default]
    Or,
}

impl BitAnd for Action {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Keep, Self::Keep) => Self::Keep,
            (Self::Keep | Self::Discard, Self::Discard) | (Self::Discard, Self::Keep) => {
                Self::Discard
            }
            (Self::AddToCorpus(name, flow_control), other) => {
                Self::AddToCorpus(name, flow_control & other)
            }
            (lhs, Self::AddToCorpus(name, flow_control)) => {
                Self::AddToCorpus(name, flow_control & lhs)
            }
        }
    }
}
impl BitAnd<FlowControl> for Action {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitand(self, rhs: FlowControl) -> Self::Output {
        match (self, rhs) {
            (Self::Keep, FlowControl::Keep) => Self::Keep,
            (Self::Keep | Self::Discard, FlowControl::Discard)
            | (Self::Discard, FlowControl::Keep) => Self::Discard,
            (Self::AddToCorpus(name, flow_control), other) => {
                Self::AddToCorpus(name, flow_control & other)
            }
        }
    }
}

impl BitAnd for FlowControl {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Keep, Self::Keep) => Self::Keep,
            (Self::Keep | Self::Discard, Self::Discard) | (Self::Discard, Self::Keep) => {
                Self::Discard
            }
        }
    }
}

impl BitAnd<Action> for FlowControl {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitand(self, rhs: Action) -> Self::Output {
        match (self, rhs) {
            (Self::Keep, Action::Keep) => Self::Keep,
            (Self::Keep | Self::Discard, Action::Discard) | (Self::Discard, Action::Keep) => {
                Self::Discard
            }
            (lhs, Action::AddToCorpus(_, flow_control)) => {
                // at this point we're just comparing two FlowControl variants, so we can
                // just use the bitwise operation
                lhs & flow_control
            }
        }
    }
}

impl BitOr for Action {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Keep | Self::Discard, Self::Keep) | (Self::Keep, Self::Discard) => Self::Keep,
            (Self::Discard, Self::Discard) => Self::Discard,
            (Self::AddToCorpus(name, flow_control), other) => {
                Self::AddToCorpus(name, flow_control | other)
            }
            (lhs, Self::AddToCorpus(name, flow_control)) => {
                Self::AddToCorpus(name, flow_control | lhs)
            }
        }
    }
}

impl BitOr<FlowControl> for Action {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitor(self, rhs: FlowControl) -> Self::Output {
        match (self, rhs) {
            (Self::Keep | Self::Discard, FlowControl::Keep)
            | (Self::Keep, FlowControl::Discard) => Self::Keep,
            (Self::Discard, FlowControl::Discard) => Self::Discard,
            (Self::AddToCorpus(name, flow_control), other) => {
                Self::AddToCorpus(name, flow_control | other)
            }
        }
    }
}

impl BitOr for FlowControl {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitor(self, rhs: Self) -> Self::Output {
        match (&self, &rhs) {
            (Self::Keep | Self::Discard, Self::Keep) | (Self::Keep, Self::Discard) => Self::Keep,
            (Self::Discard, Self::Discard) => Self::Discard,
        }
    }
}

impl BitOr<Action> for FlowControl {
    type Output = Self;

    #[instrument(level = "trace")]
    fn bitor(self, rhs: Action) -> Self::Output {
        match (self, rhs) {
            (Self::Keep | Self::Discard, Action::Keep) | (Self::Keep, Action::Discard) => {
                Self::Keep
            }
            (Self::Discard, Action::Discard) => Self::Discard,
            (lhs, Action::AddToCorpus(_, flow_control)) => {
                // at this point we're just comparing two FlowControl variants, so we can
                // just use the bitwise operation
                lhs | flow_control
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// test that the `BitAnd` implementation for `Action` produces the correct
    /// results when action is both the lhs and rhs
    #[test]
    fn test_bitand_action_and_action() {
        // action & action::keep
        assert_eq!(Action::Keep & Action::Keep, Action::Keep);
        assert_eq!(Action::Discard & Action::Keep, Action::Discard);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) & Action::Keep,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) & Action::Keep,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );

        // action & action::discard
        assert_eq!(Action::Keep & Action::Discard, Action::Discard);
        assert_eq!(Action::Discard & Action::Discard, Action::Discard);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) & Action::Discard,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) & Action::Discard,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );

        // action & addtocorpus::keep
        assert_eq!(
            Action::Keep & Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::Discard & Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep)
                & Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard)
                & Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );

        // action & addtocorpus::discard
        assert_eq!(
            Action::Keep & Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::Discard & Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep)
                & Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard)
                & Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
    }

    /// test that the `BitAnd` implementation for `Action` and `FlowControl`
    /// produces the correct results when action is the lhs and `flow_control` is the rhs
    #[test]
    fn test_bitand_action_and_flowcontrol() {
        // action & flowcontrol::keep
        assert_eq!(Action::Keep & FlowControl::Keep, Action::Keep);
        assert_eq!(Action::Discard & FlowControl::Keep, Action::Discard);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) & FlowControl::Keep,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) & FlowControl::Keep,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );

        // action & flowcontrol::discard
        assert_eq!(Action::Keep & FlowControl::Discard, Action::Discard);
        assert_eq!(Action::Discard & FlowControl::Discard, Action::Discard);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) & FlowControl::Discard,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) & FlowControl::Discard,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
    }

    /// test that the `BitAnd` implementation for `FlowControl`
    /// produces the correct results when `flow_control` is both the lhs and rhs
    #[test]
    fn test_bitand_flowcontrol_and_flowcontrol() {
        // flowcontrol & flowcontrol::keep
        assert_eq!(FlowControl::Keep & FlowControl::Keep, FlowControl::Keep);
        assert_eq!(
            FlowControl::Discard & FlowControl::Keep,
            FlowControl::Discard
        );

        // flowcontrol & flowcontrol::discard
        assert_eq!(
            FlowControl::Keep & FlowControl::Discard,
            FlowControl::Discard
        );
        assert_eq!(
            FlowControl::Discard & FlowControl::Discard,
            FlowControl::Discard
        );
    }

    /// test that the `BitAnd` implementation for `Action` and `FlowControl`
    /// produces the correct results when `flow_control` is the lhs and action is the rhs
    #[test]
    fn test_bitand_flowcontrol_and_action() {
        // flowcontrol & action::keep
        assert_eq!(FlowControl::Keep & Action::Keep, FlowControl::Keep);
        assert_eq!(FlowControl::Discard & Action::Keep, FlowControl::Discard);

        // flowcontrol & action::discard
        assert_eq!(FlowControl::Keep & Action::Discard, FlowControl::Discard);
        assert_eq!(FlowControl::Discard & Action::Discard, FlowControl::Discard);

        // flowcontrol & action::addtocorpus::keep
        assert_eq!(
            FlowControl::Keep & Action::AddToCorpus("stuff", FlowControl::Keep),
            FlowControl::Keep
        );
        assert_eq!(
            FlowControl::Discard & Action::AddToCorpus("stuff", FlowControl::Keep),
            FlowControl::Discard
        );

        // flowcontrol & action::addtocorpus::discard
        assert_eq!(
            FlowControl::Keep & Action::AddToCorpus("stuff", FlowControl::Discard),
            FlowControl::Discard
        );
        assert_eq!(
            FlowControl::Discard & Action::AddToCorpus("stuff", FlowControl::Discard),
            FlowControl::Discard
        );
    }

    // bitor operations

    /// test that the `BitOr` implementation for `Action` produces the correct
    /// results when action is both the lhs and rhs
    #[test]
    fn test_bitor_action_and_action() {
        // action | action::keep
        assert_eq!(Action::Keep | Action::Keep, Action::Keep);
        assert_eq!(Action::Discard | Action::Keep, Action::Keep);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) | Action::Keep,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) | Action::Keep,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );

        // action | action::discard
        assert_eq!(Action::Keep | Action::Discard, Action::Keep);
        assert_eq!(Action::Discard | Action::Discard, Action::Discard);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) | Action::Discard,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) | Action::Discard,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );

        // action | addtocorpus::keep
        assert_eq!(
            Action::Keep | Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::Discard | Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep)
                | Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard)
                | Action::AddToCorpus("stuff", FlowControl::Keep),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );

        // action | addtocorpus::discard
        assert_eq!(
            Action::Keep | Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::Discard | Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep)
                | Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard)
                | Action::AddToCorpus("stuff", FlowControl::Discard),
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
    }

    /// test that the `BitOr` implementation for `Action` and `FlowControl`
    /// produces the correct results when action is the lhs and `flow_control` is the rhs
    #[test]
    fn test_bitor_action_and_flowcontrol() {
        // action | flowcontrol::keep
        assert_eq!(Action::Keep | FlowControl::Keep, Action::Keep);
        assert_eq!(Action::Discard | FlowControl::Keep, Action::Keep);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) | FlowControl::Keep,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) | FlowControl::Keep,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );

        // action | flowcontrol::discard
        assert_eq!(Action::Keep | FlowControl::Discard, Action::Keep);
        assert_eq!(Action::Discard | FlowControl::Discard, Action::Discard);
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Keep) | FlowControl::Discard,
            Action::AddToCorpus("stuff", FlowControl::Keep)
        );
        assert_eq!(
            Action::AddToCorpus("stuff", FlowControl::Discard) | FlowControl::Discard,
            Action::AddToCorpus("stuff", FlowControl::Discard)
        );
    }

    /// test that the `BitOr` implementation for `FlowControl`
    /// produces the correct results when `flow_control` is both the lhs and rhs
    #[test]
    fn test_bitor_flowcontrol_and_flowcontrol() {
        // flowcontrol | flowcontrol::keep
        assert_eq!(FlowControl::Keep | FlowControl::Keep, FlowControl::Keep);
        assert_eq!(FlowControl::Discard | FlowControl::Keep, FlowControl::Keep);

        // flowcontrol | flowcontrol::discard
        assert_eq!(FlowControl::Keep | FlowControl::Discard, FlowControl::Keep);
        assert_eq!(
            FlowControl::Discard | FlowControl::Discard,
            FlowControl::Discard
        );
    }

    /// test that the `BitOr` implementation for `Action` and `FlowControl`
    /// produces the correct results when `flow_control` is the lhs and action is the rhs
    #[test]
    fn test_bitor_flowcontrol_and_action() {
        // flowcontrol | action::keep
        assert_eq!(FlowControl::Keep | Action::Keep, FlowControl::Keep);
        assert_eq!(FlowControl::Discard | Action::Keep, FlowControl::Keep);

        // flowcontrol | action::discard
        assert_eq!(FlowControl::Keep | Action::Discard, FlowControl::Keep);
        assert_eq!(FlowControl::Discard | Action::Discard, FlowControl::Discard);

        // flowcontrol | action::addtocorpus::keep
        assert_eq!(
            FlowControl::Keep | Action::AddToCorpus("stuff", FlowControl::Keep),
            FlowControl::Keep
        );
        assert_eq!(
            FlowControl::Discard | Action::AddToCorpus("stuff", FlowControl::Keep),
            FlowControl::Keep
        );

        // flowcontrol | action::addtocorpus::discard
        assert_eq!(
            FlowControl::Keep | Action::AddToCorpus("stuff", FlowControl::Discard),
            FlowControl::Keep
        );
        assert_eq!(
            FlowControl::Discard | Action::AddToCorpus("stuff", FlowControl::Discard),
            FlowControl::Discard
        );
    }
}
