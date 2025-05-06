use indicatif::ProgressDrawTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Printer {
    Silent,
    Quiet,
    Default,
    Verbose,
}

impl Printer {
    pub fn target(&self) -> ProgressDrawTarget {
        match self {
            Printer::Silent => ProgressDrawTarget::hidden(),
            Printer::Quiet => ProgressDrawTarget::hidden(),
            Printer::Default => ProgressDrawTarget::stdout(),
            Printer::Verbose => ProgressDrawTarget::hidden(),
        }
    }

    pub fn stdout(&self) -> Stdout {
        match self {
            Printer::Silent => Stdout::Disabled,
            Printer::Quiet => Stdout::Disabled,
            Printer::Default => Stdout::Enabled,
            Printer::Verbose => Stdout::Enabled,
        }
    }

    pub fn stderr(&self) -> Stderr {
        match self {
            Printer::Silent => Stderr::Disabled,
            Printer::Quiet => Stderr::Disabled,
            Printer::Default => Stderr::Enabled,
            Printer::Verbose => Stderr::Enabled,
        }
    }
}


pub enum Stdout {
    Enabled,
    Disabled,
}

impl std::fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        match self {
            Stdout::Enabled => println!("{}", s),
            Stdout::Disabled => {},
        }

        Ok(())
    }
    
}
pub enum Stderr {
    Enabled,
    Disabled,
}

impl std::fmt::Write for Stderr {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        match self {
            Stderr::Enabled => eprintln!("{}", s),
            Stderr::Disabled => {},
        }

        Ok(())
    }
}
