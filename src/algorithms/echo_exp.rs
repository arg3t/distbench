pub mod echo {
    use async_trait::async_trait;
    use common_macros::hash_map;
    use distbench::{
        self, community::PeerId, messages::AlgorithmMessage, Algorithm, SelfTerminating,
    };
    use log::{error, info};
    use std::{
        collections::HashMap,
        fmt::Display,
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };
    struct Message<T> {
        sender: String,
        message: String,
        payload: T,
    }
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<T> _serde::Serialize for Message<T>
        where
            T: _serde::Serialize,
        {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private228::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "Message",
                    false as usize + 1 + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "sender",
                    &self.sender,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "message",
                    &self.message,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "payload",
                    &self.payload,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de, T> _serde::Deserialize<'de> for Message<T>
        where
            T: _serde::Deserialize<'de>,
        {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private228::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                    __field2,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private228::Formatter,
                    ) -> _serde::__private228::fmt::Result {
                        _serde::__private228::Formatter::write_str(__formatter, "field identifier")
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private228::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private228::Ok(__Field::__field0),
                            1u64 => _serde::__private228::Ok(__Field::__field1),
                            2u64 => _serde::__private228::Ok(__Field::__field2),
                            _ => _serde::__private228::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private228::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "sender" => _serde::__private228::Ok(__Field::__field0),
                            "message" => _serde::__private228::Ok(__Field::__field1),
                            "payload" => _serde::__private228::Ok(__Field::__field2),
                            _ => _serde::__private228::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private228::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"sender" => _serde::__private228::Ok(__Field::__field0),
                            b"message" => _serde::__private228::Ok(__Field::__field1),
                            b"payload" => _serde::__private228::Ok(__Field::__field2),
                            _ => _serde::__private228::Ok(__Field::__ignore),
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private228::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de, T>
                where
                    T: _serde::Deserialize<'de>,
                {
                    marker: _serde::__private228::PhantomData<Message<T>>,
                    lifetime: _serde::__private228::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de, T> _serde::de::Visitor<'de> for __Visitor<'de, T>
                where
                    T: _serde::Deserialize<'de>,
                {
                    type Value = Message<T>;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private228::Formatter,
                    ) -> _serde::__private228::fmt::Result {
                        _serde::__private228::Formatter::write_str(__formatter, "struct Message")
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private228::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 =
                            match _serde::de::SeqAccess::next_element::<String>(&mut __seq)? {
                                _serde::__private228::Some(__value) => __value,
                                _serde::__private228::None => {
                                    return _serde::__private228::Err(
                                        _serde::de::Error::invalid_length(
                                            0usize,
                                            &"struct Message with 3 elements",
                                        ),
                                    );
                                }
                            };
                        let __field1 =
                            match _serde::de::SeqAccess::next_element::<String>(&mut __seq)? {
                                _serde::__private228::Some(__value) => __value,
                                _serde::__private228::None => {
                                    return _serde::__private228::Err(
                                        _serde::de::Error::invalid_length(
                                            1usize,
                                            &"struct Message with 3 elements",
                                        ),
                                    );
                                }
                            };
                        let __field2 = match _serde::de::SeqAccess::next_element::<T>(&mut __seq)? {
                            _serde::__private228::Some(__value) => __value,
                            _serde::__private228::None => {
                                return _serde::__private228::Err(
                                    _serde::de::Error::invalid_length(
                                        2usize,
                                        &"struct Message with 3 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private228::Ok(Message {
                            sender: __field0,
                            message: __field1,
                            payload: __field2,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private228::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private228::Option<String> =
                            _serde::__private228::None;
                        let mut __field1: _serde::__private228::Option<String> =
                            _serde::__private228::None;
                        let mut __field2: _serde::__private228::Option<T> =
                            _serde::__private228::None;
                        while let _serde::__private228::Some(__key) =
                            _serde::de::MapAccess::next_key::<__Field>(&mut __map)?
                        {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private228::Option::is_some(&__field0) {
                                        return _serde::__private228::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "sender",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private228::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private228::Option::is_some(&__field1) {
                                        return _serde::__private228::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "message",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private228::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                                __Field::__field2 => {
                                    if _serde::__private228::Option::is_some(&__field2) {
                                        return _serde::__private228::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "payload",
                                            ),
                                        );
                                    }
                                    __field2 = _serde::__private228::Some(
                                        _serde::de::MapAccess::next_value::<T>(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private228::Some(__field0) => __field0,
                            _serde::__private228::None => {
                                _serde::__private228::de::missing_field("sender")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private228::Some(__field1) => __field1,
                            _serde::__private228::None => {
                                _serde::__private228::de::missing_field("message")?
                            }
                        };
                        let __field2 = match __field2 {
                            _serde::__private228::Some(__field2) => __field2,
                            _serde::__private228::None => {
                                _serde::__private228::de::missing_field("payload")?
                            }
                        };
                        _serde::__private228::Ok(Message {
                            sender: __field0,
                            message: __field1,
                            payload: __field2,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["sender", "message", "payload"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "Message",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private228::PhantomData::<Message<T>>,
                        lifetime: _serde::__private228::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl<T: ::core::clone::Clone> ::core::clone::Clone for Message<T> {
        #[inline]
        fn clone(&self) -> Message<T> {
            Message {
                sender: ::core::clone::Clone::clone(&self.sender),
                message: ::core::clone::Clone::clone(&self.message),
                payload: ::core::clone::Clone::clone(&self.payload),
            }
        }
    }
    #[automatically_derived]
    impl<T: ::core::fmt::Debug> ::core::fmt::Debug for Message<T> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Message",
                "sender",
                &self.sender,
                "message",
                &self.message,
                "payload",
                &&self.payload,
            )
        }
    }
    impl<T: ::distbench::messages::AlgorithmMessage> ::distbench::signing::Digest for Message<T> {
        fn digest(&self) -> [u8; 32] {
            let mut hasher = ::blake3::Hasher::new();
            hasher.update(&self.sender.digest());
            hasher.update(&self.message.digest());
            hasher.update(&self.payload.digest());
            hasher.finalize().into()
        }
    }
    impl<T: ::distbench::messages::AlgorithmMessage> ::distbench::signing::Verifiable<Message<T>>
        for Message<T>
    {
        fn verify(
            self,
            keystore: &::distbench::community::KeyStore,
        ) -> Result<Self, ::distbench::PeerError> {
            {
                {
                    let lvl = ::log::Level::Trace;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api::log(
                            { ::log::__private_api::GlobalLogger },
                            format_args!("Verifying message of type {0}", "Message"),
                            lvl,
                            &(
                                "runner::algorithms::echo",
                                "runner::algorithms::echo",
                                ::log::__private_api::loc(),
                            ),
                            (),
                        );
                    }
                }
            };
            Ok(Self {
                sender: self.sender.verify(keystore)?,
                message: self.message.verify(keystore)?,
                payload: self.payload.verify(keystore)?,
            })
        }
    }
    impl<T: ::distbench::messages::AlgorithmMessage> AsRef<Message<T>> for Message<T> {
        fn as_ref(&self) -> &Message<T> {
            self
        }
    }
    impl<T: ::distbench::messages::AlgorithmMessage> ::distbench::messages::AlgorithmMessage
        for Message<T>
    {
        fn type_id(&self) -> String {
            "Message".to_string()
        }
    }
    pub struct Echo {
        start_node: bool,
        messages_received: AtomicU64,
        __id: ::distbench::community::PeerId,
        __key: ::distbench::crypto::PrivateKey,
        #[allow(dead_code)]
        __stopped_tx: ::std::sync::Arc<::tokio::sync::watch::Sender<bool>>,
        #[allow(dead_code)]
        __stopped_rx: ::tokio::sync::watch::Receiver<bool>,
        __network_size: u32,
        __connections: ::std::collections::HashMap<::distbench::community::PeerId, EchoPeer>,
    }
    impl Echo {
        fn N(&self) -> u32 {
            self.__network_size + 1
        }
        fn id(&self) -> &distbench::community::PeerId {
            &self.__id
        }
        fn peers(&self) -> impl Iterator<Item = (&distbench::community::PeerId, &EchoPeer)> {
            self.__connections
                .iter()
                .map(|(peer_id, peer)| (peer_id, peer))
        }
        fn peer(&self, id: &distbench::community::PeerId) -> Option<EchoPeer> {
            self.__connections.get(id).cloned()
        }
        fn sign<M>(&self, msg: M) -> ::distbench::signing::Signed<M>
        where
            M: ::distbench::signing::Digest
                + ::serde::Serialize
                + for<'de> ::serde::de::Deserialize<'de>,
        {
            let signature = self.__key.sign(&msg.digest());
            let id = self.__id.clone();
            ::distbench::signing::Signed::new(msg, signature, id)
        }
    }
    pub struct EchoConfig {
        pub start_node: Option<bool>,
    }
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for EchoConfig {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private228::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "EchoConfig",
                    false as usize + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "start_node",
                    &self.start_node,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for EchoConfig {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private228::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private228::Formatter,
                    ) -> _serde::__private228::fmt::Result {
                        _serde::__private228::Formatter::write_str(__formatter, "field identifier")
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private228::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private228::Ok(__Field::__field0),
                            _ => _serde::__private228::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private228::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "start_node" => _serde::__private228::Ok(__Field::__field0),
                            _ => _serde::__private228::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private228::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"start_node" => _serde::__private228::Ok(__Field::__field0),
                            _ => _serde::__private228::Ok(__Field::__ignore),
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private228::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(__deserializer, __FieldVisitor)
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private228::PhantomData<EchoConfig>,
                    lifetime: _serde::__private228::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = EchoConfig;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private228::Formatter,
                    ) -> _serde::__private228::fmt::Result {
                        _serde::__private228::Formatter::write_str(__formatter, "struct EchoConfig")
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private228::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<Option<bool>>(
                            &mut __seq,
                        )? {
                            _serde::__private228::Some(__value) => __value,
                            _serde::__private228::None => {
                                return _serde::__private228::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct EchoConfig with 1 element",
                                    ),
                                );
                            }
                        };
                        _serde::__private228::Ok(EchoConfig {
                            start_node: __field0,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private228::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private228::Option<Option<bool>> =
                            _serde::__private228::None;
                        while let _serde::__private228::Some(__key) =
                            _serde::de::MapAccess::next_key::<__Field>(&mut __map)?
                        {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private228::Option::is_some(&__field0) {
                                        return _serde::__private228::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "start_node",
                                            ),
                                        );
                                    }
                                    __field0 = _serde::__private228::Some(
                                        _serde::de::MapAccess::next_value::<Option<bool>>(
                                            &mut __map,
                                        )?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private228::Some(__field0) => __field0,
                            _serde::__private228::None => {
                                _serde::__private228::de::missing_field("start_node")?
                            }
                        };
                        _serde::__private228::Ok(EchoConfig {
                            start_node: __field0,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["start_node"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "EchoConfig",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private228::PhantomData::<EchoConfig>,
                        lifetime: _serde::__private228::PhantomData,
                    },
                )
            }
        }
    };
    impl<F, T, CM> ::distbench::AlgorithmFactory<F, T, CM> for EchoConfig
    where
        T: ::distbench::transport::Transport + 'static,
        CM: ::distbench::transport::ConnectionManager<T> + 'static,
        F: ::distbench::Format + 'static,
    {
        type Algorithm = Echo;
        fn build(
            self,
            format: ::std::sync::Arc<F>,
            key: ::distbench::crypto::PrivateKey,
            id: ::distbench::community::PeerId,
            community: ::std::sync::Arc<::distbench::community::Community<T, CM>>,
        ) -> Result<Self::Algorithm, ::distbench::ConfigError> {
            {
                {
                    let lvl = ::log::Level::Trace;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api::log(
                            { ::log::__private_api::GlobalLogger },
                            format_args!(
                                "{0}::build() - Building algorithm instance for node {1:?}",
                                "Echo", id,
                            ),
                            lvl,
                            &(
                                "runner::algorithms::echo",
                                "runner::algorithms::echo",
                                ::log::__private_api::loc(),
                            ),
                            (),
                        );
                    }
                }
            };
            let conn_managers = community.clone().neighbours();
            {
                {
                    let lvl = ::log::Level::Trace;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api::log(
                            { ::log::__private_api::GlobalLogger },
                            format_args!(
                                "{0}::build() - Creating peer proxies for {1} neighbours",
                                "Echo",
                                conn_managers.len(),
                            ),
                            lvl,
                            &(
                                "runner::algorithms::echo",
                                "runner::algorithms::echo",
                                ::log::__private_api::loc(),
                            ),
                            (),
                        );
                    }
                }
            };
            let connections: ::std::collections::HashMap<_, _> = conn_managers
                .into_iter()
                .map(|(peer_id, conn_manager)| {
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "{0}::build() - Creating peer proxy for {1:?}",
                                        "Echo", peer_id,
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    (
                        peer_id,
                        EchoPeer::new(std::sync::Arc::new(Box::new(EchoPeerInner::new(
                            conn_manager,
                            format.clone(),
                            community.clone(),
                        ))
                            as Box<dyn EchoPeerService>)),
                    )
                })
                .collect();
            let (stopped_tx, stopped_rx) = ::tokio::sync::watch::channel(false);
            {
                {
                    let lvl = ::log::Level::Trace;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api::log(
                            { ::log::__private_api::GlobalLogger },
                            format_args!(
                                "{0}::build() - Algorithm instance built successfully",
                                "Echo",
                            ),
                            lvl,
                            &(
                                "runner::algorithms::echo",
                                "runner::algorithms::echo",
                                ::log::__private_api::loc(),
                            ),
                            (),
                        );
                    }
                }
            };
            Ok(Self::Algorithm {
                start_node: self.start_node.unwrap_or(false),
                messages_received: Default::default(),
                __connections: connections,
                __network_size: community.size() as u32,
                __stopped_tx: ::std::sync::Arc::new(stopped_tx),
                __stopped_rx: stopped_rx,
                __key: key,
                __id: id,
            })
        }
    }
    impl ::distbench::algorithm::Named for Echo {
        fn name(&self) -> &str {
            "Echo"
        }
    }
    struct EchoPeerInner<
        F: ::distbench::Format,
        T: ::distbench::transport::Transport + 'static,
        CM: ::distbench::transport::ConnectionManager<T> + 'static,
    > {
        connection_manager: ::std::sync::Arc<CM>,
        format: ::std::sync::Arc<F>,
        community: ::std::sync::Arc<::distbench::community::Community<T, CM>>,
        _phantom: ::std::marker::PhantomData<T>,
    }
    #[automatically_derived]
    impl<
            F: ::core::clone::Clone + ::distbench::Format,
            T: ::core::clone::Clone + ::distbench::transport::Transport + 'static,
            CM: ::core::clone::Clone + ::distbench::transport::ConnectionManager<T> + 'static,
        > ::core::clone::Clone for EchoPeerInner<F, T, CM>
    {
        #[inline]
        fn clone(&self) -> EchoPeerInner<F, T, CM> {
            EchoPeerInner {
                connection_manager: ::core::clone::Clone::clone(&self.connection_manager),
                format: ::core::clone::Clone::clone(&self.format),
                community: ::core::clone::Clone::clone(&self.community),
                _phantom: ::core::clone::Clone::clone(&self._phantom),
            }
        }
    }
    impl<
            F: ::distbench::Format,
            T: ::distbench::transport::Transport + 'static,
            CM: ::distbench::transport::ConnectionManager<T> + 'static,
        > EchoPeerInner<F, T, CM>
    {
        pub fn new(
            connection_manager: ::std::sync::Arc<CM>,
            format: ::std::sync::Arc<F>,
            community: ::std::sync::Arc<::distbench::community::Community<T, CM>>,
        ) -> Self {
            Self {
                connection_manager,
                format,
                community,
                _phantom: ::std::marker::PhantomData,
            }
        }
    }
    struct EchoPeer(std::sync::Arc<Box<dyn EchoPeerService>>);
    #[automatically_derived]
    impl ::core::clone::Clone for EchoPeer {
        #[inline]
        fn clone(&self) -> EchoPeer {
            EchoPeer(::core::clone::Clone::clone(&self.0))
        }
    }
    impl EchoPeer {
        pub fn new(inner: ::std::sync::Arc<Box<dyn EchoPeerService>>) -> Self {
            Self(inner)
        }
    }
    impl ::distbench::SelfTerminating for Echo {
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn terminate<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                let __self = self;
                let _: () = {
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "{0}.terminate() - Sending termination signal",
                                        "Echo",
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    let _ = __self.__stopped_tx.send(true);
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "{0}.terminate() - Termination signal sent",
                                        "Echo",
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                };
            })
        }
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn terminated<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = bool> + ::core::marker::Send + 'async_trait>,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) = ::core::option::Option::None::<bool> {
                    #[allow(unreachable_code)]
                    return __ret;
                }
                let __self = self;
                let __ret: bool = {
                    let mut rx = __self.__stopped_rx.clone();
                    let _ = rx.wait_for(|stopped| *stopped).await;
                    true
                };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
    impl Algorithm for Echo {
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn on_start<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                let __self = self;
                let _: () = {
                    {
                        {
                            let lvl = ::log::Level::Info;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!("Echo algorithm starting"),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    if __self.start_node {
                        for (_, peer) in __self.peers() {
                            match peer
                                .message(&Message {
                                    sender: "Test".to_string(),
                                    message: "Hello, world!".to_string(),
                                    payload: "str".to_string(),
                                })
                                .await
                            {
                                Ok(Some(message)) => {
                                    let lvl = ::log::Level::Info;
                                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                        ::log::__private_api::log(
                                            { ::log::__private_api::GlobalLogger },
                                            format_args!("Message echoed: {0}", message),
                                            lvl,
                                            &(
                                                "runner::algorithms::echo",
                                                "runner::algorithms::echo",
                                                ::log::__private_api::loc(),
                                            ),
                                            (),
                                        );
                                    }
                                }
                                Ok(None) => {
                                    let lvl = ::log::Level::Error;
                                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                        ::log::__private_api::log(
                                            { ::log::__private_api::GlobalLogger },
                                            format_args!("Message not echoed"),
                                            lvl,
                                            &(
                                                "runner::algorithms::echo",
                                                "runner::algorithms::echo",
                                                ::log::__private_api::loc(),
                                            ),
                                            (),
                                        );
                                    }
                                }
                                Err(e) => {
                                    let lvl = ::log::Level::Error;
                                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                        ::log::__private_api::log(
                                            { ::log::__private_api::GlobalLogger },
                                            format_args!("Error echoing message: {0}", e),
                                            lvl,
                                            &(
                                                "runner::algorithms::echo",
                                                "runner::algorithms::echo",
                                                ::log::__private_api::loc(),
                                            ),
                                            (),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    __self.terminate().await;
                };
            })
        }
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn on_exit<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                let __self = self;
                let _: () = {
                    {
                        {
                            let lvl = ::log::Level::Info;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!("Echo algorithm exiting"),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                };
            })
        }
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn report<'life0, 'async_trait>(
            &'life0 self,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Option<HashMap<impl Display, impl Display>>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) =
                    ::core::option::Option::None::<Option<HashMap<_, _>>>
                {
                    #[allow(unreachable_code)]
                    return __ret;
                }
                let __self = self;
                let __ret: Option<HashMap<_, _>> = {
                    Some({
                        let start_capacity = 1;
                        #[allow(unused_mut)]
                        let mut map = ::std::collections::HashMap::with_capacity(start_capacity);
                        map.insert(
                            "messages_received",
                            __self.messages_received.load(Ordering::Relaxed).to_string(),
                        );
                        map
                    })
                };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
    #[allow(dead_code)]
    impl Echo {
        async fn message<T: AlgorithmMessage>(
            &self,
            src: PeerId,
            msg: &Message<T>,
        ) -> Option<String> {
            {
                {
                    let lvl = ::log::Level::Info;
                    if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                        ::log::__private_api::log(
                            { ::log::__private_api::GlobalLogger },
                            format_args!(
                                "Received message from {0}: {1}",
                                src.to_string(),
                                msg.message,
                            ),
                            lvl,
                            &(
                                "runner::algorithms::echo",
                                "runner::algorithms::echo",
                                ::log::__private_api::loc(),
                            ),
                            (),
                        );
                    }
                }
            };
            self.messages_received.fetch_add(1, Ordering::Relaxed);
            Some(msg.message.clone())
        }
    }
    impl<F: ::distbench::Format> ::distbench::AlgorithmHandler<F> for Echo {
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn handle<'life0, 'life1, 'async_trait>(
            &'life0 self,
            src: ::distbench::community::PeerId,
            msg_type_id: String,
            msg_bytes: Vec<u8>,
            keystore: ::distbench::community::KeyStore,
            format: &'life1 F,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<
                        Output = Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>,
                    > + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) = ::core::option::Option::None::<
                    Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>>,
                > {
                    #[allow(unreachable_code)]
                    return __ret;
                }
                let __self = self;
                let src = src;
                let msg_type_id = msg_type_id;
                let msg_bytes = msg_bytes;
                let keystore = keystore;
                let __ret: Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> = {
                    use ::distbench::signing::Verifiable;
                    use ::distbench::Format;
                    if msg_type_id == "Message<T>" {
                        {
                            {
                                let lvl = ::log::Level::Trace;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        { ::log::__private_api::GlobalLogger },
                                        format_args!(
                                            "AlgorithmHandler::handle - Deserializing message of type {0} from {1:?}",
                                            "Message<T>",
                                            src,
                                        ),
                                        lvl,
                                        &(
                                            "runner::algorithms::echo",
                                            "runner::algorithms::echo",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            }
                        };
                        let msg = format
                            .deserialize::<Message<T>>(&msg_bytes)
                            .map_err(|e| ::distbench::PeerError::DeserializationFailed {
                                message: ::alloc::__export::must_use({
                                    ::alloc::fmt::format(
                                        format_args!(
                                            "Failed to deserialize message of type \'{0}\' from {1:?}: {2}",
                                            "Message<T>",
                                            src,
                                            e,
                                        ),
                                    )
                                }),
                            })?;
                        let msg = msg.verify(&keystore)?;
                        {
                            {
                                let lvl = ::log::Level::Trace;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        { ::log::__private_api::GlobalLogger },
                                        format_args!(
                                            "AlgorithmHandler::handle - Calling handler {0} (send/reply)",
                                            "Message<T>",
                                        ),
                                        lvl,
                                        &(
                                            "runner::algorithms::echo",
                                            "runner::algorithms::echo",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            }
                        };
                        let reply: Option<String> = __self.message(src.clone(), &msg).await;
                        {
                            {
                                let lvl = ::log::Level::Trace;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        { ::log::__private_api::GlobalLogger },
                                        format_args!(
                                            "AlgorithmHandler::handle - Serializing reply of type {0}",
                                            "Option<String>",
                                        ),
                                        lvl,
                                        &(
                                            "runner::algorithms::echo",
                                            "runner::algorithms::echo",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            }
                        };
                        let reply_bytes = format.serialize(&reply).map_err(|e| {
                            ::distbench::PeerError::SerializationFailed {
                                message: ::alloc::__export::must_use({
                                    ::alloc::fmt::format(format_args!(
                                        "Failed to serialize reply of type \'{0}\': {1}",
                                        "Option<String>", e,
                                    ))
                                }),
                            }
                        })?;
                        {
                            {
                                let lvl = ::log::Level::Trace;
                                if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                    ::log::__private_api::log(
                                        { ::log::__private_api::GlobalLogger },
                                        format_args!(
                                            "AlgorithmHandler::handle - Handler {0} completed, reply: {1} bytes",
                                            "Message<T>",
                                            reply_bytes.len(),
                                        ),
                                        lvl,
                                        &(
                                            "runner::algorithms::echo",
                                            "runner::algorithms::echo",
                                            ::log::__private_api::loc(),
                                        ),
                                        (),
                                    );
                                }
                            }
                        };
                        return Ok(Some(reply_bytes));
                    }
                    Err(::distbench::PeerError::UnknownMessageType {
                        message: ::alloc::__export::must_use({
                            ::alloc::fmt::format(format_args!(
                                "Received unhandled message type \'{0}\'",
                                msg_type_id,
                            ))
                        }),
                    }
                    .into())
                };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
    impl EchoPeer {
        async fn message(
            &self,
            msg: impl AsRef<Message<T>>,
        ) -> Result<Option<String>, ::distbench::PeerError> {
            self.0.message(msg.as_ref()).await
        }
    }
    trait EchoPeerService: Send + Sync {
        #[must_use]
        #[allow(
            elided_named_lifetimes,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds
        )]
        fn message<'life0, 'life1, 'async_trait>(
            &'life0 self,
            msg: &'life1 Message<T>,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<Option<String>, ::distbench::PeerError>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait;
    }
    impl<F, T, CM> EchoPeerService for EchoPeerInner<F, T, CM>
    where
        F: ::distbench::Format,
        T: ::distbench::transport::Transport,
        CM: ::distbench::transport::ConnectionManager<T>,
    {
        #[allow(
            elided_named_lifetimes,
            clippy::async_yields_async,
            clippy::diverging_sub_expression,
            clippy::let_unit_value,
            clippy::needless_arbitrary_self_type,
            clippy::no_effect_underscore_binding,
            clippy::shadow_same,
            clippy::type_complexity,
            clippy::type_repetition_in_bounds,
            clippy::used_underscore_binding
        )]
        fn message<'life0, 'life1, 'async_trait>(
            &'life0 self,
            msg: &'life1 Message<T>,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<Option<String>, ::distbench::PeerError>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            'life1: 'async_trait,
            Self: 'async_trait,
        {
            Box::pin(async move {
                if let ::core::option::Option::Some(__ret) =
                    ::core::option::Option::None::<Result<Option<String>, ::distbench::PeerError>>
                {
                    #[allow(unreachable_code)]
                    return __ret;
                }
                let __self = self;
                let __ret: Result<Option<String>, ::distbench::PeerError> = {
                    use ::distbench::signing::Verifiable;
                    use ::distbench::transport::ConnectionManager;
                    use ::distbench::Format;
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "Peer::{0} - Serializing message of type {1}",
                                        "message", "Message<T>",
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    let msg_bytes = __self.format.serialize(msg).map_err(|e| {
                        ::distbench::PeerError::SerializationFailed {
                            message: ::alloc::__export::must_use({
                                ::alloc::fmt::format(format_args!(
                                    "Failed to serialize message of type \'{0}\': {1}",
                                    "Message<T>", e,
                                ))
                            }),
                        }
                    })?;
                    let envelope =
                        ::distbench::NodeMessage::Algorithm("Message<T>".to_string(), msg_bytes);
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "Peer::{0} - Serializing envelope with rkyv",
                                        "message",
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    let envelope_bytes = ::rkyv::to_bytes::<_, 256>(&envelope).map_err(|e| {
                        ::distbench::PeerError::SerializationFailed {
                            message: ::alloc::__export::must_use({
                                ::alloc::fmt::format(format_args!(
                                    "Failed to serialize envelope with rkyv: {0}",
                                    e,
                                ))
                            }),
                        }
                    })?;
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "Peer::{0} - Sending {1} bytes and waiting for reply",
                                        "message",
                                        envelope_bytes.len(),
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    let reply_bytes = __self
                        .connection_manager
                        .send(envelope_bytes.to_vec())
                        .await
                        .map_err(|e| ::distbench::PeerError::TransportError {
                            message: ::alloc::__export::must_use({
                                ::alloc::fmt::format(format_args!("Failed to send message: {0}", e))
                            }),
                        })?;
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!(
                                        "Peer::{0} - Received reply: {1} bytes, deserializing",
                                        "message",
                                        reply_bytes.len(),
                                    ),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    let reply: Option<String> =
                        __self.format.deserialize(&reply_bytes).map_err(|e| {
                            ::distbench::PeerError::DeserializationFailed {
                                message: ::alloc::__export::must_use({
                                    ::alloc::fmt::format(format_args!(
                                        "Failed to deserialize reply of type \'{0}\': {1}",
                                        "Option<String>", e,
                                    ))
                                }),
                            }
                        })?;
                    let reply = reply.verify(&__self.community.keystore())?;
                    {
                        {
                            let lvl = ::log::Level::Trace;
                            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                                ::log::__private_api::log(
                                    { ::log::__private_api::GlobalLogger },
                                    format_args!("Peer::{0} - Send/reply complete", "message"),
                                    lvl,
                                    &(
                                        "runner::algorithms::echo",
                                        "runner::algorithms::echo",
                                        ::log::__private_api::loc(),
                                    ),
                                    (),
                                );
                            }
                        }
                    };
                    Ok(reply)
                };
                #[allow(unreachable_code)]
                __ret
            })
        }
    }
}
