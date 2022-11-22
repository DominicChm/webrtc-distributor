// Powers the internal server
use rouille::{router, Response};

pub fn init() {
    rouille::start_server("localhost:80", move |request| {
        router!(request,
            // first route
            (GET) (/) => {
                serve_index()
            },

            // second route
            (GET) (/hello) => {
                Response::text("Howdy")
            },

            // default route
            _ => Response::text("Endpoint not found").with_status_code(400)
        )
    })
}

fn serve_index() -> Response {
    if cfg!(debug_assertions) {
        Response::from_file(
            "text/html",
            std::fs::File::open("public/index.html").expect("Unable to read index file."),
        )
    } else {
        Response::html(include_str!("../public/index.html"))
    }
}
