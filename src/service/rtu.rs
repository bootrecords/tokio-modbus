use crate::frame::{rtu::*, *};
use crate::proto::rtu::Proto;

use futures::{future, Future};
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::BindClient;
use tokio_serial::Serial;
use tokio_service::Service;

/// Modbus RTU client
pub struct Client {
    service: ClientService<Serial, Proto>,
    slave_addr: SlaveAddress,
}

impl Client {
    /// Establish a serial connection with a Modbus server.
    pub fn bind(
        handle: &Handle,
        serial: Serial,
        slave_addr: SlaveAddress,
    ) -> impl Future<Item = Self, Error = Error> {
        let proto = Proto;
        let service = proto.bind_client(handle, serial);
        future::ok(Self {
            service,
            slave_addr,
        })
    }

    fn next_request_adu<R>(&self, req: R) -> RequestAdu
    where
        R: Into<RequestPdu>,
    {
        let slave_addr = self.slave_addr;
        let hdr = Header { slave_addr };
        let pdu = req.into();
        RequestAdu { hdr, pdu }
    }
}

fn verify_response_header(req_hdr: Header, rsp_hdr: Header) -> Result<(), Error> {
    if req_hdr != rsp_hdr {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!(
                "Invalid response header: expected/request = {:?}, actual/response = {:?}",
                req_hdr, rsp_hdr
            ),
        ));
    }
    Ok(())
}

impl Service for Client {
    type Request = Request;
    type Response = Response;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let req_adu = self.next_request_adu(req);
        let req_hdr = req_adu.hdr;

        let result = self
            .service
            .call(req_adu)
            .and_then(move |rsp_adu| match rsp_adu.pdu {
                ResponsePdu(Ok(rsp)) => verify_response_header(req_hdr, rsp_adu.hdr).and(Ok(rsp)),
                ResponsePdu(Err(err)) => Err(Self::Error::new(ErrorKind::Other, err)),
            });

        Box::new(result)
    }
}
