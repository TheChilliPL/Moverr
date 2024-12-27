pub enum FileSizeUnit {
    Byte,
    KibiByte,
    MebiByte,
    GibiByte,
    TebiByte,
    PebiByte,
    ExbiByte,
    // Too big to use anyway --- bytes don't fit in u64
    // ZebiByte,
    // YobiByte,
}

impl FileSizeUnit {
    pub fn to_bytes(&self) -> u64 {
        match self {
            FileSizeUnit::Byte => 1,
            FileSizeUnit::KibiByte => 1 << 10,
            FileSizeUnit::MebiByte => 1 << 20,
            FileSizeUnit::GibiByte => 1 << 30,
            FileSizeUnit::TebiByte => 1 << 40,
            FileSizeUnit::PebiByte => 1 << 50,
            FileSizeUnit::ExbiByte => 1 << 60,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            FileSizeUnit::Byte => 'B',
            FileSizeUnit::KibiByte => 'K',
            FileSizeUnit::MebiByte => 'M',
            FileSizeUnit::GibiByte => 'G',
            FileSizeUnit::TebiByte => 'T',
            FileSizeUnit::PebiByte => 'P',
            FileSizeUnit::ExbiByte => 'E',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'B' => Some(FileSizeUnit::Byte),
            'K' => Some(FileSizeUnit::KibiByte),
            'M' => Some(FileSizeUnit::MebiByte),
            'G' => Some(FileSizeUnit::GibiByte),
            'T' => Some(FileSizeUnit::TebiByte),
            'P' => Some(FileSizeUnit::PebiByte),
            'E' => Some(FileSizeUnit::ExbiByte),
            _ => None,
        }
    }

    pub fn to_acronym(self) -> &'static str {
        match self {
            FileSizeUnit::Byte => "B",
            FileSizeUnit::KibiByte => "KiB",
            FileSizeUnit::MebiByte => "MiB",
            FileSizeUnit::GibiByte => "GiB",
            FileSizeUnit::TebiByte => "TiB",
            FileSizeUnit::PebiByte => "PiB",
            FileSizeUnit::ExbiByte => "EiB",
        }
    }

    pub fn from_acronym(acronym: &str) -> Option<Self> {
        match acronym {
            "B" => Some(FileSizeUnit::Byte),
            "KiB" => Some(FileSizeUnit::KibiByte),
            "MiB" => Some(FileSizeUnit::MebiByte),
            "GiB" => Some(FileSizeUnit::GibiByte),
            "TiB" => Some(FileSizeUnit::TebiByte),
            "PiB" => Some(FileSizeUnit::PebiByte),
            "EiB" => Some(FileSizeUnit::ExbiByte),
            _ => None,
        }
    }

    pub fn to_name_singular(self) -> &'static str {
        match self {
            FileSizeUnit::Byte => "byte",
            FileSizeUnit::KibiByte => "kibibyte",
            FileSizeUnit::MebiByte => "mebibyte",
            FileSizeUnit::GibiByte => "gibibyte",
            FileSizeUnit::TebiByte => "tebibyte",
            FileSizeUnit::PebiByte => "pebibyte",
            FileSizeUnit::ExbiByte => "exbibyte",
        }
    }

    pub fn to_name_plural(self) -> &'static str {
        match self {
            FileSizeUnit::Byte => "bytes",
            FileSizeUnit::KibiByte => "kibibytes",
            FileSizeUnit::MebiByte => "mebibytes",
            FileSizeUnit::GibiByte => "gibibytes",
            FileSizeUnit::TebiByte => "tebibytes",
            FileSizeUnit::PebiByte => "pebibytes",
            FileSizeUnit::ExbiByte => "exbibytes",
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            FileSizeUnit::Byte => Some(FileSizeUnit::KibiByte),
            FileSizeUnit::KibiByte => Some(FileSizeUnit::MebiByte),
            FileSizeUnit::MebiByte => Some(FileSizeUnit::GibiByte),
            FileSizeUnit::GibiByte => Some(FileSizeUnit::TebiByte),
            FileSizeUnit::TebiByte => Some(FileSizeUnit::PebiByte),
            FileSizeUnit::PebiByte => Some(FileSizeUnit::ExbiByte),
            FileSizeUnit::ExbiByte => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            FileSizeUnit::Byte => None,
            FileSizeUnit::KibiByte => Some(FileSizeUnit::Byte),
            FileSizeUnit::MebiByte => Some(FileSizeUnit::KibiByte),
            FileSizeUnit::GibiByte => Some(FileSizeUnit::MebiByte),
            FileSizeUnit::TebiByte => Some(FileSizeUnit::GibiByte),
            FileSizeUnit::PebiByte => Some(FileSizeUnit::TebiByte),
            FileSizeUnit::ExbiByte => Some(FileSizeUnit::PebiByte),
        }
    }
}
