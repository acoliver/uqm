//! Mode-side adapters lending a conversation to the race scripts.

#[cfg(test)]
mod clock_tests;

pub(crate) mod alliance;
#[cfg(not(test))]
pub(crate) mod arilou;
#[cfg(not(test))]
pub(crate) mod blackur;
#[cfg(not(test))]
pub(crate) mod chmmr;
#[cfg(not(test))]
pub(crate) mod comandr;
#[cfg(not(test))]
pub(crate) mod druuge;
pub(crate) mod encounter;
#[cfg(not(test))]
pub(crate) mod ilwrath;
#[cfg(not(test))]
pub(crate) mod melnorm;
#[cfg(not(test))]
pub(crate) mod mycon;
#[cfg(not(test))]
pub(crate) mod orz;
#[cfg(not(test))]
pub(crate) mod pkunk;
#[cfg(not(test))]
pub(crate) mod rebel;
pub(crate) mod script;
pub(crate) mod shofixt;
#[cfg(not(test))]
pub(crate) mod slyhome;
#[cfg(not(test))]
pub(crate) mod slyland;
#[cfg(not(test))]
pub(crate) mod spahome;
#[cfg(not(test))]
pub(crate) mod spathi;
#[cfg(not(test))]
pub(crate) mod starbas;
#[cfg(not(test))]
pub(crate) mod supox;
#[cfg(not(test))]
pub(crate) mod syreen;
#[cfg(not(test))]
pub(crate) mod talkpet;
#[cfg(not(test))]
pub(crate) mod thradd;
#[cfg(not(test))]
pub(crate) mod umgah;
#[cfg(not(test))]
pub(crate) mod urquan;
#[cfg(not(test))]
pub(crate) mod utwig;
#[cfg(not(test))]
pub(crate) mod vux;
pub(crate) mod yehat;
#[cfg(not(test))]
pub(crate) mod zoqfot;
