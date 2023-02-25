use log::LevelFilter;
use log4rs::{
    append::rolling_file::{
        policy::compound::roll::delete::DeleteRoller, policy::compound::trigger::size::SizeTrigger,
        policy::compound::CompoundPolicy, RollingFileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let retention = Box::new(CompoundPolicy::new(
        Box::new(SizeTrigger::new(30e7 as u64)),
        Box::new(DeleteRoller::new()),
    ));

    let logfile = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}\n")))
        .build("log/fips.log", retention)?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    Ok(())
}
