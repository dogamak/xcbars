error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Timer(::tokio_timer::TimerError);
    }

    errors {
        XcbConnection(err: ::xcb::ConnError) {
            description("xbc connection error")
            display("error while connecting to xorg: {:?}", err)
        }
        XcbGenericError(err: u8) {
            description("xcb error")
            display("xcb error code {}", err)
        }
        ItemError {
            description("item error")
            display("item error")
        }
    }
}

impl From<::xcb::GenericError> for Error {
    fn from(err: ::xcb::GenericError) -> Error {
        ErrorKind::XcbGenericError(err.error_code()).into()
    }
}
