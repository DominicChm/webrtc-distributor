// Generates a config file. Just here so I don't have to manually mess w/ json
let s = JSON.stringify({
    streams: [
        {
            port: 5002,
            ip: "239.7.69.7",   
            id: "Test Stream",  // Stream ID. Should be unique
            default: false,     // Added by default when a new client connects?
        }
    ]
});

require("fs").writeFileSync("./config.json", s);