// Powers the internal server
use rouille::{router, Response, session, Request};

pub struct APIRx {

}

pub fn init() {
    rouille::start_server("localhost:80", move |request| {
        router!(request,
            // first route
            (GET) (/) => {
                serve_index(&request)
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

fn serve_index(request: &Request) -> Response {
    session::session(request, "SID", 3600, |session| {
        let id: &str = session.id();

        // This id is unique to each client.

        Response::text(format!("Session ID: {}", id))

        // if cfg!(debug_assertions) {
        //     Response::from_file(
        //         "text/html",
        //         std::fs::File::open("public/index.html").expect("Unable to read index file."),
        //     )
        // } else {
        //     Response::html(include_str!("../public/index.html"))
        // }
    })
}
