use seahorse::{Flag, FlagType};

pub fn choice_flag() -> Flag {
    Flag::new("choice", FlagType::Bool).description("disable alternatives system")
}

pub fn debug_flag() -> Flag {
    Flag::new("debug", FlagType::Bool)
        .description("print debug information")
        .alias("d")
}

pub fn force_flag() -> Flag {
    Flag::new("force", FlagType::Bool)
        .description("force install package(s)")
        .alias("f")
}

pub fn prompt_flag() -> Flag {
    Flag::new("prompt", FlagType::Bool).description("disable prompts")
}

pub fn strip_flag() -> Flag {
    Flag::new("strip", FlagType::Bool).description("disable package stripping")
}

pub fn quiet_flag() -> Flag {
    Flag::new("quiet", FlagType::Bool)
        .description("do not print build logs")
        .alias("q")
}

pub fn verbose_flag() -> Flag {
    Flag::new("verbose", FlagType::Bool)
        .description("print advanced information")
        .alias("v")
}

pub fn pid_flag() -> Flag {
    Flag::new("process-id", FlagType::Int)
        .description("set pid(not recommended unless you are using it for testing)")
        .alias("pid")
}

pub fn kiss_compress_flag() -> Flag {
    Flag::new("kiss-compress", FlagType::String)
        .description("Compression method to use for built package tarballs.(default: gz)")
        .alias("compress")
}

pub fn kiss_root_flag() -> Flag {
    Flag::new("kiss-root", FlagType::String)
        .description("Where installed packages will go.(default: '/')")
        .alias("root")
}

pub fn kiss_cache_dir_flag() -> Flag {
    Flag::new("kiss-cache-dir", FlagType::String)
        .description("Where package binaries/sources will be.(default: '${XDG_CACHE_HOME:-$HOME/.cache}/kiss')")
        .alias("cache-dir")
        .alias("cache")
}

pub fn kiss_tmp_dir_flag() -> Flag {
    Flag::new("kiss-tmp-dir", FlagType::String)
        .description(
            "Where packages will be built.(default: '${XDG_CACHE_HOME:-$HOME/.cache}/kiss')",
        )
        .alias("tmp-dir")
        .alias("tmp")
}

pub fn kiss_path_flag() -> Flag {
    Flag::new("kiss-path", FlagType::String)
        .description("List of repositories. This works exactly like '$PATH'(seperated by ':')")
        .alias("path")
}

// this depends on threading feature
pub fn jobs_flag() -> Flag {
    #[cfg(feature = "threading")]
    return Flag::new("jobs", FlagType::Int)
        .description(
            "Number of cores that will be used for threaded operations(disabled by default)",
        )
        .alias("j");
    #[cfg(not(feature = "threading"))]
    return Flag::new("jobs", FlagType::Int)
        .description("feature disabled at compile-time")
        .alias("j");
}
