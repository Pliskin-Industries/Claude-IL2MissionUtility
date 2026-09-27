//! Mission-log replay CLI entry point; replay is built in step 0T.

pub fn run_cli(args: &[String]) -> Option<i32> {
    let cmd = args.first()?.as_str();
    if cmd != "replay" {
        return None;
    }
    eprintln!("replay is built in step 0T");
    Some(2)
}
