#[macro_export]
macro_rules! tauri_reexport {
    (
        impl $t:ty {$(
            pub fn $method:ident($self:ident: $self_type:ty$(, $($argument:ident: $argument_type:ty),*)?) -> $return_type:ty $body:block
        )*}
    ) => {
        impl $t {$(
            #[allow(clippy::too_many_arguments)]
            pub fn $method($self: $self_type$(, $($argument: $argument_type),*)?) -> $return_type $body
        )*}

        $(
            #[tauri::command]
            #[allow(clippy::too_many_arguments)]
            pub fn $method(arg1: tauri::State<std::sync::Arc<std::sync::Mutex<$t>>>$(, $($argument: $argument_type),*)?) -> $return_type {
                arg1.lock().unwrap().$method($($($argument),*)?)
            }
        )*
    };
}

pub(super) use tauri_reexport;