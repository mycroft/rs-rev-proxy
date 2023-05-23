use futures_util::stream::Stream;
use log::{debug, error};

use rusoto_core::ByteStream;

use std::{
    fs::OpenOptions,
    io::{Result, Seek, SeekFrom, Write},
    pin::Pin,
    task::{Context, Poll},
};

pub struct PouetStream {
    inner_stream: ByteStream,
    fp: String,
    offset: usize,
    completed: bool
}

impl PouetStream
{
    pub fn new(stream: ByteStream, fp: String) -> Self
    {
        Self {
            inner_stream: stream,
            fp,
            offset: 0,
            completed: false,
        }
    }
}

impl Drop for PouetStream {
    fn drop(&mut self) {
        if !self.completed {
            let _ = std::fs::remove_file(self.fp.clone());
        }
    }
}

impl Stream for PouetStream
{
    type Item = Result<bytes::Bytes>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        let myref = self.get_mut();
        let res = Pin::new(&mut myref.inner_stream).poll_next(cx);

        match res {
            Poll::Pending => {
                debug!("Poll is pending...");
                Poll::Pending
            }
            Poll::Ready(x) => match x {
                None => {
                    debug!("poll::ready::none");
                    myref.completed = true;
                    Poll::Ready(None)
                },
                Some(chunk) => {
                    match chunk {
                        Ok(data) => {
                            let file = OpenOptions::new()
                                .read(true)
                                .write(true)
                                .create(true)
                                .open(myref.fp.clone());

                            if file.is_err() {
                                let err = file.err().unwrap();
                                error!("Polling is pending because of {:?}...", err);
                                return Poll::Ready(Some(Err(err)));
                            }

                            let mut file = file.unwrap();
                            if let Err(err) = file.seek(SeekFrom::Start(myref.offset as u64)) {
                                return Poll::Ready(Some(Err(err)));
                            }
                            if let Err(err) = file.write_all(&data) {
                                return Poll::Ready(Some(Err(err)));
                            }

                            myref.offset += data.len();
                            
                            debug!("writing some data: {}", data.len());
                            Poll::Ready(Some(Ok(data)))
                        },
                        Err(err) => {
                            error!("poll_next::err: {:?}", err);
                            Poll::Ready(Some(Err(err)))
                        }
                    }
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner_stream.size_hint()
    }
}
