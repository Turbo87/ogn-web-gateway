use std::io;
use std::time::Duration;

use actix::actors::{Connect, Connector};
use actix::prelude::*;
use actix::io::FramedWrite;
use tokio_core::net::TcpStream;
use tokio_io::codec::{FramedRead, LinesCodec};
use tokio_io::io::WriteHalf;
use tokio_io::AsyncRead;

use regex::Regex;

/// Received a position record from the OGN client.
#[derive(Message, Clone)]
pub struct OGNRecord {
    pub record: OGNPositionRecord,
}

#[derive(Clone)]
pub struct OGNPositionRecord {
    pub raw: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl OGNPositionRecord {
    /// Parse latitude and longitude from the raw APRS record
    pub fn try_parse(raw: &str) -> Option<OGNPositionRecord> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"([/@])(\d{6}h)(\d{2})(\d{2}.\d{2})([NS]).(\d{2})(\d{3}.\d{2})([EW])").unwrap();
        }

        RE.captures(raw).map(|cap| {
            let latitude = (cap.get(3).unwrap().as_str().parse::<f64>().unwrap()
                + cap.get(4).unwrap().as_str().parse::<f64>().unwrap() / 60.)
                * if cap.get(4).unwrap().as_str() == "S" { -1. } else { 1. };

            let longitude = (cap.get(6).unwrap().as_str().parse::<f64>().unwrap()
                + cap.get(7).unwrap().as_str().parse::<f64>().unwrap() / 60.)
                * if cap.get(8).unwrap().as_str() == "W" { -1. } else { 1. };

            OGNPositionRecord { raw: raw.to_string(), latitude, longitude }
        })
    }
}

/// An actor that connects to the [OGN](https://www.glidernet.org/) APRS servers
pub struct OGNClient {
    recipient: Recipient<Syn, OGNRecord>,
    cell: Option<FramedWrite<WriteHalf<TcpStream>, LinesCodec>>,
}

impl OGNClient {
    pub fn new(recipient: Recipient<Syn, OGNRecord>) -> OGNClient {
        OGNClient { recipient, cell: None }
    }

    /// Schedule sending a "keep alive" message to the server every 30sec
    fn schedule_keepalive(ctx: &mut Context<Self>) {
        ctx.run_later(Duration::from_secs(30), |act, ctx| {
            println!("sending keepalive");
            if let Some(ref mut framed) = act.cell {
                framed.write("# keep alive".to_string());
            }
            OGNClient::schedule_keepalive(ctx);
        });
    }
}

impl Actor for OGNClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("Connected to the OGN server");

        Connector::from_registry()
            .send(Connect::host("glidern1.glidernet.org:10152"))
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(stream) => {
                    println!("Connected to OGN server");

                    let (r, w) = stream.split();

                    // configure write side of the connection
                    let mut framed = actix::io::FramedWrite::new(w, LinesCodec::new(), ctx);

                    // send login message
                    framed.write("user test pass -1 vers test 1.0".to_string());

                    // save writer for later
                    act.cell = Some(framed);

                    // read side of the connection
                    ctx.add_stream(FramedRead::new(r, LinesCodec::new()));

                    // schedule sending a "keep alive" message to the server every 30sec
                    OGNClient::schedule_keepalive(ctx);
                }
                Err(err) => {
                    println!("Can not connect to OGN server: {}", err);
                    ctx.stop();
                }
            })
            .map_err(|err, _act, ctx| {
                println!("Can not connect to OGN server: {}", err);
                ctx.stop();
            })
            .wait(ctx);
    }

    fn stopped(&mut self, _: &mut Context<Self>) {
        println!("Disconnected from OGN server");
    }
}

impl actix::io::WriteHandler<io::Error> for OGNClient {}

/// Parse received lines into `OGNPositionRecord` instances
/// and send them to the `recipient`
impl StreamHandler<String, io::Error> for OGNClient {
    fn handle(&mut self, msg: String, _: &mut Self::Context) {
        if let Some(record) = OGNPositionRecord::try_parse(&msg) {
            self.recipient.do_send(OGNRecord { record });
        }
    }
}
