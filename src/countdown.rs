use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::env::Args;
use crate::countdown::CountdownCommand::*;
use crate::countdown::countdown_serializer::{CountdownService, CountdownError};
use std::fs::File;
use std::io::{Write, Read};
use std::{fs, io};
use std::ops::Add;
use regex::Regex;

mod countdown_serializer;

#[derive(Clone)]
pub struct Countdown {
    name: String,
    end_time: SystemTime
}

impl Countdown {
    pub fn time_remaining(&self, now: SystemTime) -> Duration {
        return if self.end_time > now {
            self.end_time.duration_since(now).unwrap()
        } else {
            Duration::from_secs(0)
        }
    }
}

#[derive(Debug)]
pub enum CountdownCommand {
    DisplayAll,
    AddNew{ name: String, duration: Duration },
    Cancel { name: String }
}

impl CountdownCommand {
    fn from_args(command: String, args: &mut Args) -> CountdownCommand {
        match command.as_str() {
            "add" => {
                let mut str_args = args.collect::<Vec<String>>();
                let dur_str = str_args.remove(str_args.len() - 1);
                let name = str_args.join(" ");
                let duration = parse_dur_str(&dur_str);
                AddNew { name: name, duration: duration }
            },
            "cancel" => {
                let name = args.collect::<Vec<String>>().join(" ");
                Cancel { name: name }
            },
            _ => { DisplayAll }
        }
    }
}

impl From<Args> for CountdownCommand {
    fn from(mut args: Args) -> Self {
        match args.next() {
            Some(command) => CountdownCommand::from_args(command, &mut args),
            None => DisplayAll
        }
    }
}

pub struct CountdownStore {}

impl CountdownService for CountdownStore {

    fn save(&mut self, cd: Countdown) -> Result<(), CountdownError> {
        let mut file = File::create(format!("{}.timer", cd.name.as_str()))?;
        file.write_all(self.serialize_time(&cd.end_time).as_ref())?;
        Ok(())
    }

    fn load(&mut self, name: &str) -> Result<Countdown, CountdownError> {
        let path = format!("./{}", name);
        let mut file = File::open(path)?;
        let mut time_string = String::new();
        file.read_to_string(&mut time_string)?;
        let time = self.deserialize_time(&time_string);

        Ok(Countdown{name: name.to_string(), end_time: time})
    }

    fn delete(&mut self, name: &str) -> Result<(), CountdownError> {
        fs::remove_file(format!("./{}.timer", name))?;
        Ok(())
    }

    fn list(&mut self) -> Result<Vec<String>, CountdownError> {
        let mut entries = fs::read_dir(".")?
            .map(|res| res.map(|e| e.path().file_name().unwrap().to_str().unwrap().to_string()))
            .filter(|f| f.as_ref().ok().is_some() && f.as_ref().unwrap().ends_with(".timer"))
            .collect::<Result<Vec<String>, io::Error>>()?;

        // The order in which `read_dir` returns entries is not guaranteed. If reproducible
        // ordering is required the entries should be explicitly sorted.
        entries.sort();

        Ok(entries)
    }
}

impl CountdownStore {
    fn serialize_time(&self, time: &SystemTime) -> String {
        let unix_time = time.duration_since(UNIX_EPOCH).unwrap();
        unix_time.as_secs().to_string()
    }

    fn deserialize_time(&self, timestr: &String) -> SystemTime {
        let timestamp: u64 = timestr.parse::<u64>().unwrap();
        SystemTime::UNIX_EPOCH.add(Duration::from_secs(timestamp))
    }
}

pub struct Counterdowner {
    store: Box<dyn CountdownService>
}

impl Counterdowner {
    pub fn new(service: Box<dyn CountdownService>) -> Counterdowner{
        Counterdowner{ store: service }
    }

    pub fn execute_countdown(&mut self, cmd: CountdownCommand) -> Result<String, CountdownError> {
        match cmd {
            AddNew { name, duration } => { self.add_timer(name.as_str(), duration) },
            Cancel { name } => { self.cancel_timer(name.as_str()) }
            DisplayAll => { self.display_timers() }
        }
    }
    pub fn timers(&mut self) -> Vec<String> {
        self.store.list().unwrap_or(vec![])
    }

    pub fn add_timer(&mut self, name: &str, duration: Duration) -> Result<String, CountdownError> {
        let cd = Countdown { name: String::from(name), end_time: SystemTime::now() + duration };
        self.store.save(cd)?;
        Ok(format!("Added Timer: {}", name))
    }

    pub fn cancel_timer(&mut self, name: &str) -> Result<String, CountdownError> {
        self.store.delete(name)?;
        Ok(format!("Canceled timer {}", name))
    }

    pub fn display_timers(&mut self) -> Result<String, CountdownError> {
        let mut out = String::new();

        for file in self.store.list()? {
            let cd = self.store.load(file.as_str())?;
            out += Counterdowner::format_countdown(&cd).as_str();
        }

        Ok(out)
    }

    fn format_countdown(cd: &Countdown) -> String {
        let time_remaining = cd.time_remaining(SystemTime::now());
        let hours_remaining = time_remaining.as_secs() / 3600;
        let minutes_remaining = time_remaining.as_secs() / 60 - hours_remaining * 60;
        let seconds_remaining = time_remaining.as_secs() - hours_remaining * 3600 - minutes_remaining * 60;

        format!("{}: {:02}:{:02}:{:02}", cd.name, hours_remaining, minutes_remaining, seconds_remaining)
    }
}

type TimeUnit = (Regex, i32);

fn parse_dur_str(dur_str: &String) -> Duration {
    let hours: Regex = Regex::new("(\\d+)[hH]").unwrap();
    let minutes: Regex = Regex::new("(\\d+)[mM]").unwrap();
    let seconds: Regex = Regex::new("(\\d+)[sS]").unwrap();

    let timeunits: Vec<TimeUnit> = vec![(hours, 3600), (minutes, 60), (seconds, 1)];

    let mut dur_s = 0;

    for (re, scalar) in timeunits {
        let captures = re.captures(dur_str);
        if captures.is_none() { continue; }

        let units = captures.unwrap().get(1).unwrap().as_str().parse::<i32>().unwrap();
        dur_s += scalar * units;
    }

    Duration::from_secs(dur_s as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::countdown::countdown_serializer::CountdownError;
    use std::collections::HashMap;

    #[test]
    fn count_down_calculates_correct_remaining_time() {
        let now = SystemTime::now();
        let cd = Countdown{
            name: String::from("test timer"),
            end_time: now + Duration::from_secs(10)
        };

        assert_eq!(cd.time_remaining(now), Duration::from_secs(10));
    }

    #[test]
    fn count_down_returns_zero_if_past_time() {
        let now = SystemTime::now();
        let cd = Countdown{
            name: String::from("test timer"),
            end_time: now - Duration::from_secs(10) // ten seconds ago
        };

        assert_eq!(cd.time_remaining(now), Duration::from_secs(0));
    }

    /****
    Counterdowner Tests
     */
    struct CounterdownerSpy{
        pub countdowns: HashMap<String, Countdown>
    }

    #[test]
    fn add_new_creates_a_new_countdown() {
        let name = "timer";
        let duration = Duration::from_secs(30);
        let mut ctr = Counterdowner::new(Box::new(CounterdownerSpy::new()));

        ctr.add_timer(name, duration);

        assert_eq!(ctr.timers().contains(&name.to_string()), true);
    }

    #[test]
    fn cancel_removes_created_countdown() {
        let name = "timer";
        let duration = Duration::from_secs(30);
        let mut ctr = Counterdowner::new(Box::new(CounterdownerSpy::new()));

        ctr.add_timer(name, duration);
        assert_eq!(ctr.timers().contains(&name.to_string()), true);

        ctr.cancel_timer(name);
        assert_eq!(ctr.timers().contains(&name.to_string()), false);
    }

    #[test]
    fn display_prints_out_timers() {
        let name = "timer";
        let duration = Duration::from_secs(30);
        let mut ctr = Counterdowner::new(Box::new(CounterdownerSpy::new()));

        ctr.add_timer(name, duration);

        let res = ctr.display_timers();

        assert!(res.is_ok());
        assert_eq!(res.unwrap_or("".to_string()),
        "timer: 00:00:29"
        )
    }

    impl CounterdownerSpy {
        pub fn new() -> CounterdownerSpy {
            CounterdownerSpy { countdowns: HashMap::new() }
        }
    }

    impl CountdownService for CounterdownerSpy {
        fn save(&mut self, cd: Countdown) -> Result<(), CountdownError> {
            self.countdowns.insert(cd.name.clone(), cd);
            Ok(())
        }

        fn load(&mut self, name: &str) -> Result<Countdown, CountdownError> {
            match self.countdowns.get(name) {
                None => Err(CountdownError::NotFound(name.to_string())),
                Some(cd) => { Ok(cd.clone()) }
            }
        }

        fn delete(&mut self, name: &str) -> Result<(), CountdownError> {
            self.countdowns.remove(name);
            Ok(())
        }

        fn list(&mut self) -> Result<Vec<String>, CountdownError> {
            Ok(self.countdowns.keys().map(|k| k.to_string()).collect())
        }
    }
}
