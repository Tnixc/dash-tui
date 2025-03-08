//! `NetworkTables` data values and types.

use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

// holy macro
macro_rules! impl_data_type {
    // T (and vec<T>) to data type with from<data type> -> T impl
    ($t: ty $([vec => $a: expr])? => $d: expr ; $v: ident @ $f: expr) => {
        impl_data_type!(@ $t, $d, [$v]{ $f });
        $( impl_data_type!(vec $t => $a; $v @ { $f }); )?
    };

    // T (and vec<T>) to data type with from<data type> -> T impl (block)
    ($t: ty $([vec => $a: expr])? => $d: expr ; $v: ident @ $f: block) => {
        impl_data_type!(@ $t, $d, [$v]$f);
        $( impl_data_type!(vec $t => $a; $v @ { $f }); )?
    };

    // int (and vec<int>) to data type
    ($t: ty $([vec => $a: expr])? => $d: expr ; i) => {
        impl_data_type!(@ $t, $d, [value] value.as_i64());
        $( impl_data_type!(vec $t => $a; value @ { value.as_i64().and_then(|value| value.try_into().ok()) }); )?
    };
    // uint (and vec<uint>) to data type
    ($t: ty $([vec => $a: expr])? => $d: expr ; u) => {
        impl_data_type!(@ $t, $d, [value] value.as_u64());
        $( impl_data_type!(vec $t => $a; value @ { value.as_u64().and_then(|value| value.try_into().ok()) }); )?
    };

    // some_wrapper(vec<T>) to data type with mapper from value to T
    (vec($i: ty) $t: ty => $d: expr ; $v: ident @ $c: block) => {
        impl_data_type!(@ $t, $d, [value]{
            let vec = value.as_array()?;
            vec.iter()
                .map(|$v| $c)
                .collect::<Option<$t>>()
        }, [this]{
            let vec: Vec<$i> = this.into();
            rmpv::Value::from_iter(vec)
        });
    };
    // vec<T> to data type with mapper from value to T
    (vec $i: ty => $d: expr ; $v: ident @ $c: block) => {
        impl_data_type!(@ Vec<$i>, $d, [value]{
            let vec = value.as_array()?;
            vec.iter()
                .map(|$v| $c)
                .collect::<Option<Vec<$i>>>()
        }, [this]{ rmpv::Value::from_iter(this) });
    };
    // some_wrapper(vec<u8>) to data type
    (bytes $t: ty => $d: expr) => {
        impl_data_type!(vec(u8) $t => $d ; value @ {
            value.as_u64().and_then(|value| value.try_into().ok())
        });
    };

    // INTERNAL generic impl with some from logic without custom into logic
    (@ $t: ty, $d: expr, [ $v: ident ] $a: expr) => {
        impl_data_type!(@ $t, $d, [$v]{ $a.and_then(|value| value.try_into().ok()) }, [this]{ this.into() });
    };
    // INTERNAL generic impl with custom from and into logic
    (@ $t: ty, $d: expr, [ $v: ident ] $f: block, [ $s: ident ] $i: block) => {
        impl NetworkTableData for $t {
            fn data_type() -> DataType {
                use DataType::*;
                $d
            }
            #[allow(unused_variables)]
            fn from_value($v: &rmpv::Value) -> Option<Self> {
                $f
            }
            fn into_value(self) -> rmpv::Value {
                let $s = self;
                $i
            }
        }
    };
}

macro_rules! transparent {
    ($(#[$m: meta])* $t: ident : $g: ty) => {
        transparent!(@ $t, $($m)*, $g);
    };
    ($(#[$m: meta])* $t: ident : vec $g: ty) => {
        transparent!(@ $t, $($m)*, Vec<$g>);
        transparent!(@vec $t, $g);
    };

    (@ $t: ident, $($m: meta)*, $g: ty) => {
        $(#[$m])*
        pub struct $t(pub $g);

        impl From<$t> for $g {
            fn from(value: $t) -> Self {
                value.0
            }
        }
        impl From<$g> for $t {
            fn from(value: $g) -> Self {
                Self(value)
            }
        }

        #[allow(clippy::from_over_into)]
        impl Into<rmpv::Value> for $t {
            fn into(self) -> rmpv::Value {
                self.0.into()
            }
        }
    };
    (@vec $t: ident, $i: ty) => {
        impl FromIterator<$i> for $t {
            fn from_iter<I: IntoIterator<Item = $i>>(iter: I) -> Self {
                iter.into_iter().collect::<Vec<$i>>().into()
            }
        }
    };
}

/// A data type understood by a `NetworkTables` server.
#[derive(Debug, Clone, PartialEq, Serialize, Eq)]
pub enum DataType {
    /// Struct schema data type
    StructSchema,
    /// A struct type with a name (e.g., "struct:Rotation2d")
    Struct(String),
    /// [`bool`] data type
    Boolean,
    /// [`f64`] data type
    Double,
    /// Any integer data type
    Int,
    /// [`f32`] data type
    Float,
    /// [`String`] data type
    String,
    /// JSON data type
    Json,
    /// Raw binary data type
    Raw,
    /// RPC data type
    Rpc,
    /// MessagePack data type
    Msgpack,
    /// Google Protocol Buffers data type
    Protobuf,
    /// [`Vec<bool>`] data type
    BooleanArray,
    /// [`Vec<f64>`] data type
    DoubleArray,
    /// A [`Vec`] of integers data type
    IntArray,
    /// [`Vec<f32>`] data type
    FloatArray,
    /// [`Vec<String>`] data type
    StringArray,
    /// Any other type
    Other(String),
}

impl<'de> Deserialize<'de> for DataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First, deserialize the value as a string
        let value = String::deserialize(deserializer)?;

        // Now match the string to the appropriate enum variant
        match value.as_str() {
            s if s.starts_with("struct:") => {
                // Extract the name part after "struct:"
                let name = s.trim_start_matches("struct:").to_string();
                Ok(DataType::Struct(name))
            }
            "boolean" => Ok(DataType::Boolean),
            "double" => Ok(DataType::Double),
            "int" => Ok(DataType::Int),
            "float" => Ok(DataType::Float),
            "string" => Ok(DataType::String),
            "json" => Ok(DataType::Json),
            "raw" => Ok(DataType::Raw),
            "rpc" => Ok(DataType::Rpc),
            "msgpack" => Ok(DataType::Msgpack),
            "protobuf" => Ok(DataType::Protobuf),
            "boolean[]" => Ok(DataType::BooleanArray),
            "double[]" => Ok(DataType::DoubleArray),
            "int[]" => Ok(DataType::IntArray),
            "float[]" => Ok(DataType::FloatArray),
            "string[]" => Ok(DataType::StringArray),
            "structschema" => Ok(DataType::StructSchema),
            _ => Ok(DataType::Other(value)),
        }
    }
}
impl DataType {
    /// Creates a new `DataType` from an id.
    ///
    /// Returns [`Option::None`] if no `DataType` could be found with that id.
    ///
    /// It is guaranteed that the id mappings here match with the id mappings
    /// in [`as_id`](Self::as_id).
    pub fn from_id(id: u32) -> Option<Self> {
        use DataType as D;

        match id {
            0 => Some(D::Boolean),
            1 => Some(D::Double),
            2 => Some(D::Int),
            3 => Some(D::Float),
            4 => Some(D::String),
            5 => Some(D::Raw),
            16 => Some(D::BooleanArray),
            17 => Some(D::DoubleArray),
            18 => Some(D::IntArray),
            19 => Some(D::FloatArray),
            20 => Some(D::StringArray),

            _ => None,
        }
    }

    /// Returns this `DataType` as an id.
    ///
    /// It is guaranteed that the id mappings here match with the id mappings in
    /// [`from_id`](Self::from_id).
    pub fn as_id(&self) -> u32 {
        use DataType as D;

        match self {
            D::Boolean => 0,
            D::Double => 1,
            D::Int => 2,
            D::Float => 3,
            D::String | D::Json => 4,
            D::Raw | D::Rpc | D::Msgpack | D::Protobuf => 5,
            D::BooleanArray => 16,
            D::DoubleArray => 17,
            D::IntArray => 18,
            D::FloatArray => 19,
            D::StringArray => 20,
            _ => u32::MAX,
        }
    }
}

/// A piece of data that can be sent and received by a `NetworkTables` server.
pub trait NetworkTableData: Clone {
    /// Returns the `DataType` that this piece of data is.
    fn data_type() -> DataType;

    /// Creates a new piece of data from a generic `MessagePack` value.
    fn from_value(value: &rmpv::Value) -> Option<Self>;

    /// Converts this into a generic `MessagePack` value.
    fn into_value(self) -> rmpv::Value;
}

transparent!(
    /// A JSON string.
    #[derive(Clone, Debug)]
    JsonString: String
);
transparent!(
    /// Raw binary data.
    #[derive(Clone, Debug)]
    RawData: vec u8
);
transparent!(
    /// Raw RPC data.
    #[derive(Clone, Debug)]
    Rpc: vec u8
);
transparent!(
    /// Raw protobuf data.
    #[derive(Clone, Debug)]
    Protobuf: vec u8
);

impl_data_type!(bool [vec => BooleanArray] => Boolean; value @ value.as_bool());
impl_data_type!(f64 [vec => DoubleArray] => Double; value @ value.as_f64());
impl_data_type!(i8 [vec => IntArray] => Int; i);
impl_data_type!(i16 [vec => IntArray] => Int; i);
impl_data_type!(i32 [vec => IntArray] => Int; i);
impl_data_type!(i64 [vec => IntArray] => Int; i);
impl_data_type!(u8 [vec => IntArray] => Int; u);
impl_data_type!(u16 [vec => IntArray] => Int; u);
impl_data_type!(u32 [vec => IntArray] => Int; u);
impl_data_type!(u64 [vec => IntArray] => Int; u);
impl_data_type!(f32 [vec => FloatArray] => Float; value @ value.as_f64().map(|num| num as f32));
impl_data_type!(String [vec => StringArray] => String; value @ value.as_str().map(|str| str.to_owned()));
impl_data_type!(JsonString => Json; value @ value.as_str().map(|str| JsonString(str.to_owned())));
impl_data_type!(bytes RawData => Raw);
impl_data_type!(bytes Rpc => Rpc);
impl_data_type!(rmpv::Value => Msgpack; value @ Some(value.clone()));
impl_data_type!(bytes Protobuf => Protobuf);

pub(super) fn serialize_as_u32<S>(data_type: &DataType, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u32(data_type.as_id())
}

pub(super) fn deserialize_u32<'de, D>(deserializer: D) -> Result<DataType, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_u32(DataTypeVisitor)
}

pub(super) struct DataTypeVisitor;

impl<'de> Visitor<'de> for DataTypeVisitor {
    type Value = DataType;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a valid type id")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_u64(v.try_into().map_err(E::custom)?)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        DataType::from_id(v.try_into().map_err(E::custom)?)
            .ok_or(E::custom(format!("{v} is not a valid type id")))
    }
}
