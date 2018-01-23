extern crate futures;
extern crate hyper;
extern crate native_tls;
extern crate tokio_core;

#[macro_use]
extern crate configure;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod ntlm;

use ntlm::connector::NtlmProxyConnector;
use futures::{Future, Stream};
use hyper::{Uri, Client, Request};
use hyper::server::{self, Service, Http};
use native_tls::TlsConnector;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpListener;
use std::str::FromStr;
use std::net::SocketAddr;
use configure::Configure;
use hyper::error::Error;

#[derive(Deserialize, Configure)]
#[serde(default)]
pub struct Config {
    pub url: &'static str
}

impl Default for Config {
    fn default() -> Config {
        Config {
            url: "http://127.0.0.1:8888"
        }
    }
}

struct Proxy {
    handle: Handle,
}

impl Service for Proxy {
    type Request = server::Request;
    type Response = server::Response;
    type Error = Error;
    type Future = Box<Future<Item=Self::Response, Error = Error>>;

    fn call(&self, req: server::Request) -> Self::Future {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let config = Config::generate().unwrap();
        let tls_connector = TlsConnector::builder().unwrap().build().unwrap();
        let proxy = Uri::from_str(config.url).unwrap();
        let connector = NtlmProxyConnector::new(tls_connector, proxy, &self.handle);
        let client = Client::configure().connector(connector).build(&self.handle);
        let mut client_req = Request::new(method, uri);
        client_req.headers_mut().extend(req.headers().iter());
        client_req.set_body(req.body());
        let resp = client.request(client_req)
                         .then(move |result| {
                             match result {
                                 Ok(client_resp) => {
                                     Ok(server::Response::new()
                                            .with_status(client_resp.status())
                                            .with_headers(client_resp.headers().clone())
                                            .with_body(client_resp.body()))
                                 }
                                 Err(e) => {
                                     println!("{:?}", &e);
                                     Err(e)
                                 }
                             }
                         });
        Box::new(resp)
    }
}

fn main () {
    let srv_addr: SocketAddr = "0.0.0.0:8888".parse().unwrap();

    let http = Http::new();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let listener = TcpListener::bind(&srv_addr, &handle).unwrap();
    let server = listener.incoming()
                            .for_each(|(sock, addr)| {
                                let service = Proxy { handle: handle.clone() };
                                http.bind_connection(&handle, sock, addr, service);
                                Ok(())
                            });

    core.run(server).unwrap();


//   let client = Client::configure()
//       .connector(connector)
//       .build(&handle);
//   let uri_https = "https://web.site".parse().unwrap();
//   let req_https = Request::new(Method::Get, uri_https);
  
//   let work = client.request(req_https).and_then(|res| {
//       res.body().for_each(|chunk| {
//           io::stdout()
//               .write_all(&chunk)
//               .map(|_| ())
//               .map_err(From::from)
//       })
//   });

//   println!("Making request");
//   let work_result = core.run(work);
//   println!("Work result: {:?}", work_result);
//   match work_result {
//       Ok(result) => println!("Successfully retrieved a result, {:?}", result),
//       Err(error) => println!("Got an error: {:?}", error),
//   };
}