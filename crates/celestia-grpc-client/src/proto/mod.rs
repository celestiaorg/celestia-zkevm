pub mod celestia {
    pub mod zkism {
        pub mod v1 {
            include!("celestia.zkism.v1.rs");
        }
    }
}

pub mod cosmos {
    pub mod base {
        pub mod v1beta1 {
            include!("cosmos.base.v1beta1.rs");
        }
        pub mod query {
            pub mod v1beta1 {
                include!("cosmos.base.query.v1beta1.rs");
            }
        }
    }
}

pub mod hyperlane {
    pub mod core {
        pub mod v1 {
            include!("hyperlane.core.v1.rs");
        }
    }
    pub mod warp {
        pub mod v1 {
            include!("hyperlane.warp.v1.rs");
        }
    }
}
