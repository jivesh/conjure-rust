// Copyright 2021 Palantir Technologies, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//! Support for double collection keys.
use ordered_float::OrderedFloat;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};

/// A wrapper type allowing `f64` to be used as a key in collection types.
///
/// Conjure allows `map<double, T>` and `set<double>`, but Rust's `f64` type does not implement `Eq` and `Ord`,
/// preventing the direct translations of `BTreeMap<f64, T>` and `BTreeSet<f64>` from compiling. This wrapper type is
/// used to provide suitable trait implementations. The code generated by `conjure-codegen` will use this type,
/// resulting in `BTreeMap<DoubleKey<f64>, T>` and `BTreeSet<DoubleKey<f64>>`.
///
/// All trait implementations delegate directly to the inner type, with the exception of the `PartialEq`, `Eq`,
/// `PartialOrd`, and `Ord` methods.
#[derive(Debug, Copy, Clone, Default)]
pub struct DoubleKey(pub f64);

impl Display for DoubleKey {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for DoubleKey {
    type Target = f64;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DoubleKey {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialOrd for DoubleKey {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        OrderedFloat(self.0).partial_cmp(&OrderedFloat(other.0))
    }
}

impl PartialEq for DoubleKey {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        OrderedFloat(self.0) == OrderedFloat(other.0)
    }
}

impl Ord for DoubleKey {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        OrderedFloat(self.0).cmp(&OrderedFloat(other.0))
    }
}

impl Eq for DoubleKey {}

impl Hash for DoubleKey {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        OrderedFloat(self.0).hash(state)
    }
}

impl<'de> Deserialize<'de> for DoubleKey {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        f64::deserialize(deserializer).map(DoubleKey)
    }
}

impl Serialize for DoubleKey {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}
