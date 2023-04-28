mod container;
mod install;
mod kube;
mod network;
pub mod service;
pub mod unit;
mod volume;

#[cfg(unix)]
mod systemd_dbus;

use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use color_eyre::{
    eyre::{self, Context},
    Help,
};

use crate::quadlet;

use self::{
    container::Container, install::Install, kube::Kube, network::Network, service::Service,
    unit::Unit, volume::Volume,
};

#[allow(clippy::option_option)]
#[derive(Parser, Debug, Clone, PartialEq)]
#[command(author, version, about, subcommand_precedence_over_arg = true)]
pub struct Cli {
    /// Generate a file instead of printing to stdout
    ///
    /// Optionally provide a path for the file,
    /// if no path is provided the file will be placed in the current working directory.
    ///
    /// If not provided, the name of the generated file will be taken from,
    /// the `name` parameter for volumes and networks,
    /// the filename of the kube file,
    /// the container name,
    /// or the name of the container image.
    #[arg(short, long, group = "file_out")]
    file: Option<Option<PathBuf>>,

    /// Generate a file in the podman unit directory instead of printing to stdout
    ///
    /// Conflicts with the --file option
    ///
    /// Equivalent to `--file $XDG_CONFIG_HOME/containers/systemd/` for non-root users,
    /// or `--file /etc/containers/systemd/` for root.
    ///
    /// The name of the file can be specified with the --name option.
    #[arg(
        short,
        long,
        visible_alias = "unit-dir",
        conflicts_with = "file",
        group = "file_out"
    )]
    unit_directory: bool,

    /// Override the name of the generated file (without the extension)
    ///
    /// This only applies if a file was not given to the --file option,
    /// or the --unit-directory option was used.
    ///
    /// E.g. `podlet --file --name hello-world podman run quay.io/podman/hello`
    /// will generate a file with the name "hello-world.container".
    #[arg(short, long, requires = "file_out")]
    name: Option<String>,

    /// Overwrite existing files when generating a file
    ///
    /// By default, podlet will return an error if a file already exists at the given location.
    #[arg(long, alias = "override", requires = "file_out")]
    overwrite: bool,

    /// Skip the check for existing services of the same name
    ///
    /// By default, podlet will check for existing services with the same name as
    /// the service quadlet will generate from the generated quadlet file
    /// and return an error if a conflict is found.
    /// This option will cause podlet to skip that check.
    #[arg(long, requires = "file_out")]
    skip_services_check: bool,

    /// The \[Unit\] section
    #[command(flatten)]
    unit: Unit,

    /// The \[Install\] section
    #[command(flatten)]
    install: Install,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn print_or_write_files(self) -> eyre::Result<()> {
        if self.unit_directory || self.file.is_some() {
            let path = self.file_path()?;
            if matches!(path, FilePath::Full(..))
                && matches!(self.command, Commands::Compose { .. })
            {
                return Err(eyre::eyre!(
                    "A file path was provided to `--file` and the `compose` command was used"
                )
                .suggestion(
                    "Provide a directory to `--file`. \
                        `compose` can generate multiple files so a directory is needed.",
                ));
            }

            let overwrite = self.overwrite;
            #[cfg(unix)]
            let services_check = !self.skip_services_check;

            let quadlet_files = self.into_quadlet_files();

            #[cfg(unix)]
            if services_check {
                for file in &quadlet_files {
                    match &path {
                        FilePath::Full(path) => {
                            if let Some(name) = path.file_stem().and_then(OsStr::to_str) {
                                let service = file.resource.name_to_service(name);
                                check_existing(name, &service, overwrite)?;
                            }
                        }
                        FilePath::Dir(_) => {
                            check_existing(&file.name, &file.service_name(), overwrite)?;
                        }
                    }
                }
            }

            for file in quadlet_files {
                let path: Cow<Path> = match &path {
                    FilePath::Full(path) => path.into(),
                    FilePath::Dir(path) => {
                        let mut path = path.clone();
                        path.push(&file.name);
                        path.set_extension(file.resource.extension());
                        path.into()
                    }
                };
                file.write(&path, overwrite)?;
            }

            Ok(())
        } else {
            let quadlet_files = self
                .into_quadlet_files()
                .into_iter()
                .map(|file| format!("# {}.{}\n{file}", file.name, file.resource.extension()))
                .collect::<Vec<_>>()
                .join("\n---\n");
            print!("{quadlet_files}");
            Ok(())
        }
    }

    /// Returns the file path for the generated file
    fn file_path(&self) -> eyre::Result<FilePath> {
        let path = if self.unit_directory {
            #[cfg(unix)]
            if nix::unistd::Uid::current().is_root() {
                let path = PathBuf::from("/etc/containers/systemd/");
                if path.is_dir() {
                    path
                } else {
                    PathBuf::from("/usr/share/containers/systemd/")
                }
            } else {
                let mut path: PathBuf = env::var("XDG_CONFIG_HOME")
                    .or_else(|_| env::var("HOME").map(|home| format!("{home}/.config")))
                    .unwrap_or_else(|_| String::from("~/.config/"))
                    .into();
                path.push("containers/systemd/");
                path
            }

            #[cfg(not(unix))]
            return Err(eyre::eyre!(
                "Cannot get podman unit directory on non-unix system"
            ));
        } else if let Some(Some(path)) = &self.file {
            if path.is_dir() {
                path.clone()
            } else {
                return Ok(FilePath::Full(path.clone()));
            }
        } else {
            env::current_dir()
                .wrap_err("File path not provided and can't access current directory")?
        };

        Ok(FilePath::Dir(path))
    }

    fn into_quadlet_files(self) -> Vec<quadlet::File> {
        let unit = (!self.unit.is_empty()).then_some(self.unit);
        let install = self.install.install.then(|| self.install.into());
        match self.command {
            Commands::Podman { command } => {
                let service = command.service().cloned();
                let file = quadlet::File {
                    name: self.name.unwrap_or_else(|| String::from(command.name())),
                    unit,
                    resource: command.into(),
                    service,
                    install,
                };
                vec![file]
            }
            Commands::Compose { pod, compose_file } => todo!(),
        }
    }
}

#[derive(Debug)]
enum FilePath {
    Full(PathBuf),
    Dir(PathBuf),
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
enum Commands {
    /// Generate a podman quadlet file from a podman command
    Podman {
        #[command(subcommand)]
        command: PodmanCommands,
    },

    /// Generate podman quadlet files from a compose file
    ///
    /// Creates a `.container` file for each service,
    /// a `.volume` file for each volume,
    /// and a `.network` file for each network.
    ///
    /// The `--file` option must be a directory if used.
    ///
    /// Some compose options are not supported, such as `build`.
    ///
    /// When podlet encounters an unsupported option, a warning will be emitted on stderr.
    Compose {
        /// Create a Kubernetes YAML file for a pod instead of separate containers
        ///
        /// A `.kube` file using the generated Kubernetes YAML file will also be created.
        #[arg(long)]
        pod: Option<String>,

        /// The compose file to convert
        ///
        /// If not provided, podlet will look for (in order)
        /// `compose.yaml`, `compose.yml`, `docker-compose.yaml`, and `docker-compose.yml`,
        /// in the current working directory.
        compose_file: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
enum PodmanCommands {
    /// Generate a podman quadlet `.container` file
    ///
    /// For details on options see:
    /// https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html
    Run {
        /// The \[Container\] section
        #[command(flatten)]
        container: Box<Container>,

        /// The \[Service\] section
        #[command(flatten)]
        service: Service,
    },

    /// Generate a podman quadlet `.kube` file
    ///
    /// For details on options see:
    /// https://docs.podman.io/en/latest/markdown/podman-kube-play.1.html
    Kube {
        /// The \[Kube\] section
        #[command(subcommand)]
        kube: Kube,
    },

    /// Generate a podman quadlet `.network` file
    ///
    /// For details on options see:
    /// https://docs.podman.io/en/latest/markdown/podman-network-create.1.html
    Network {
        /// The \[Network\] section
        #[command(subcommand)]
        network: Network,
    },

    /// Generate a podman quadlet `.volume` file
    ///
    /// For details on options see:
    /// https://docs.podman.io/en/latest/markdown/podman-volume-create.1.html
    Volume {
        /// The \[Volume\] section
        #[command(subcommand)]
        volume: Volume,
    },
}

impl From<PodmanCommands> for quadlet::Resource {
    fn from(value: PodmanCommands) -> Self {
        match value {
            PodmanCommands::Run { container, .. } => (*container).into(),
            PodmanCommands::Kube { kube } => kube.into(),
            PodmanCommands::Network { network } => network.into(),
            PodmanCommands::Volume { volume } => volume.into(),
        }
    }
}

impl PodmanCommands {
    fn service(&self) -> Option<&Service> {
        match self {
            Self::Run { service, .. } => (!service.is_empty()).then_some(service),
            _ => None,
        }
    }

    /// Returns the name that should be used for the generated file
    fn name(&self) -> &str {
        match self {
            Self::Run { container, .. } => container.name(),
            Self::Kube { kube } => kube.name(),
            Self::Network { network } => network.name(),
            Self::Volume { volume } => volume.name(),
        }
    }
}

#[cfg(unix)]
fn check_existing(name: &str, service: &str, overwrite: bool) -> eyre::Result<()> {
    if let Ok(unit_files) = systemd_dbus::unit_files() {
        for systemd_dbus::UnitFile { file_name, status } in unit_files {
            if !(overwrite && status == "generated") && file_name.contains(service) {
                return Err(eyre::eyre!(
                    "File name `{name}` conflicts with existing unit file: {file_name}"
                )
                .suggestion(
                    "Change the generated file's name with `--file` or `--name`. \
                                Alternatively, use `--skip-services-check` if this is ok.",
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
}
