wit_bindgen::generate!({
    world: "filter-world",
});

use crate::river::request::logger::info;

struct MyModule;

impl Guest for MyModule {
    //false not means the request will be blocked (idk why)
    fn filter(req: Request) -> bool {
        if req.path == "/hubabuba" {
            info("hubabuba is filtered!");
            return true
        }
        false
    }
}

export!(MyModule);
