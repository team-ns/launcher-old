use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn configure() {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} {h([{l} {M}])}: {m}{n}",
        )))
        .build();

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} {h([{l} {M}])}: {m}{n}",
        )))
        .append(false)
        .build("launcher.log")
        .expect("Can't build logfile manager");

    let root_builder = Root::builder().appender("logfile").appender("stdout");

    let mut config_builder = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)));

    let mut config = if cfg!(debug_assertions) {
        config_builder
            .logger(Logger::builder().build("rustyline", LevelFilter::Info))
            .build(root_builder.build(LevelFilter::Debug))
    } else {
        config_builder.build(root_builder.build(LevelFilter::Info))
    };
    log4rs::init_config(config.expect("Can't build logger")).expect("Can't init logger");
}
