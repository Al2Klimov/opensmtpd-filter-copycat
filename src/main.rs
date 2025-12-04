mod util;

use mail_parser::MessageParser;
use regex_automata::meta::Regex;
use regex_syntax::hir::{Hir, Look};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write, stderr, stdin, stdout};
use util::join_write_bytes;

fn main() -> std::io::Result<()> {
    let mut std_in = stdin().lock();
    let mut std_out = stdout().lock();
    let mut std_err = stderr().lock();

    let mut line = Vec::<u8>::new();
    let mut sessions = HashMap::<Vec<u8>, (Vec<Vec<u8>>, Vec<u8>)>::new();

    loop {
        line.clear();
        std_in.read_until(b'\n', &mut line)?;

        if line.is_empty() {
            return Ok(());
        }

        while line
            .pop_if(|last| match last {
                b'\r' => true,
                b'\n' => true,
                _ => false,
            })
            .is_some()
        {}

        let mut fields = line.split(|&sep| sep == b'|');

        match fields.next() {
            Some(b"config") => match fields.next() {
                Some(b"ready") => {
                    writeln!(std_out, "register|report|smtp-in|tx-begin")?;
                    writeln!(std_out, "register|report|smtp-in|tx-rcpt")?;
                    writeln!(std_out, "register|filter|smtp-in|data-line")?;
                    writeln!(std_out, "register|filter|smtp-in|commit")?;
                    writeln!(std_out, "register|report|smtp-in|link-disconnect")?;
                    writeln!(std_out, "register|ready")?;
                }
                _ => {}
            },
            Some(b"report") => {
                fields.next(); // protocol version
                fields.next(); // timestamp
                fields.next(); // subsystem

                match (fields.next(), fields.next()) {
                    (Some(phase), Some(session)) => match phase {
                        b"tx-begin" => {
                            sessions.insert(session.to_owned(), Default::default());
                        }
                        b"tx-rcpt" => match (fields.next(), fields.next(), fields.next()) {
                            (Some(_), Some(b"ok"), Some(rcpt)) => match sessions.get_mut(session) {
                                None => {}
                                Some((rcpts, _)) => rcpts.push(rcpt.to_owned()),
                            },
                            _ => {}
                        },
                        b"link-disconnect" => {
                            sessions.remove(session);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            Some(b"filter") => {
                fields.next(); // protocol version
                fields.next(); // timestamp
                fields.next(); // subsystem

                match (fields.next(), fields.next(), fields.next()) {
                    (Some(phase), Some(session), Some(token)) => match phase {
                        b"data-line" => {
                            std_out.write_all(b"filter-dataline|")?;
                            std_out.write_all(session)?;
                            std_out.write_all(b"|")?;
                            std_out.write_all(token)?;
                            std_out.write_all(b"|")?;

                            join_write_bytes(&mut std_out, b"|", fields.clone())?;
                            writeln!(std_out, "")?;

                            let mut flds = fields.clone();

                            match (flds.next(), flds.next()) {
                                (Some(b"."), None) => {}
                                _ => match sessions.get_mut(session) {
                                    None => {}
                                    Some((_, mail)) => {
                                        join_write_bytes(mail, b"|", fields)?;
                                        writeln!(mail, "")?;
                                    }
                                },
                            }
                        }
                        b"commit" => {
                            std_out.write_all(b"filter-result|")?;
                            std_out.write_all(session)?;
                            std_out.write_all(b"|")?;
                            std_out.write_all(token)?;

                            writeln!(
                                std_out,
                                "|{}",
                                if match sessions.get(session) {
                                    None => true,
                                    Some((rcpts, mail)) =>
                                        match MessageParser::new().parse_headers(mail) {
                                            None => {
                                                writeln!(std_err, "Malformed eMail:")?;
                                                std_err.write_all(mail)?;
                                                writeln!(std_err, ".")?;
                                                true
                                            }
                                            Some(mail) =>
                                                match mail.from() {
                                                    None => true,
                                                    Some(from) => {
                                                        let mut allow = true;
                                                        let mut domains = HashSet::<&[u8]>::new();

                                                        for rcpt in rcpts {
                                                            match rcpt
                                                                .rsplitn(2, |&sep| sep == b'@')
                                                                .next()
                                                            {
                                                                None => {}
                                                                Some(domain) => {
                                                                    if domains.insert(domain) {
                                                                        match Regex::builder().build_from_hir(&Hir::concat(vec![
                                                                            Hir::look(Look::WordUnicode),
                                                                            Hir::literal(domain),
                                                                            Hir::look(Look::WordUnicode),
                                                                        ])) {
                                                                            Err(err) => {
                                                                                write!(std_err, "Couldn't build regex from recipient domain '")?;
                                                                                std_err.write_all(domain)?;
                                                                                writeln!(std_err, "': {}", err)?;
                                                                            }
                                                                            Ok(rgx) => {
                                                                                for addr in from.iter() {
                                                                                    match addr.name {
                                                                                        None => {}
                                                                                        Some(ref name) => {
                                                                                            if rgx.find(name.as_ref()).is_some() {
                                                                                                write!(std_err, "Sender name contains recipient domain '")?;
                                                                                                std_err.write_all(domain)?;
                                                                                                writeln!(std_err, "': {}", name)?;

                                                                                                allow = false;
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        allow
                                                    }
                                                },
                                        },
                                } {
                                    writeln!(std_err, "Allowing")?;
                                    "proceed"
                                } else {
                                    writeln!(std_err, "Denying")?;
                                    "reject|550 Sender name contains recipient domain"
                                }
                            )?;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }
}
