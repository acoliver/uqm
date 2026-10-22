use super::abi::{NetplayOptions, NetplayPeerOptions};

#[no_mangle]
pub static mut netplayOptions: NetplayOptions = NetplayOptions {
    metaServer: c"uqm.stack.nl".as_ptr(),
    metaPort: c"21836".as_ptr(),
    peer: [
        NetplayPeerOptions {
            isServer: true,
            host: c"localhost".as_ptr(),
            port: c"21837".as_ptr(),
        },
        NetplayPeerOptions {
            isServer: true,
            host: c"localhost".as_ptr(),
            port: c"21837".as_ptr(),
        },
    ],
    inputDelay: 2,
};
