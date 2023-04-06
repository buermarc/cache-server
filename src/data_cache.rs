use actix::prelude::*;
use rand::prelude::*;

#[derive(Message)]
#[rtype(result = "isize")]
pub struct RandU;

pub struct DataCache {
    pub rand: isize
}

impl DataCache {
    pub fn new() -> Self {
        DataCache { rand: random() }
    }
}

impl Actor for DataCache {
    type Context = Context<Self>;
}

impl Handler<RandU> for DataCache {
    type Result = isize;

    fn handle(&mut self, _msg: RandU, _ctx: &mut Context<Self>) -> Self::Result {
        self.rand
    }

}
