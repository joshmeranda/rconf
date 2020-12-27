# rconf
Package, distribute, and deploy system configurations simply and easily.

## Configuration
Rconf allows you to specify specific files to package into the archive, as well as the package manager to use for
installing, upgrading, and uninstalling packages. You can see below a list and description of each configuration key
value pair in the following sections. 

The configuration is done using the [TOML](https://toml.io/) format and all entries must follow the specification.

### Paths
Specifies the paths to store in the archive.

| name | type | purpose |
| ---- | ---- | ------- |
| `paths.home` | Array | an array of paths relative to the users home directory |
| `paths.config` | Array | an array of paths relative to the users configuration direcotyr (typically `.config`) |
| `paths.absolute` | Array | an array of absolute paths. These may point to the same locations as in the other sections, but will be less concise |

### Manager
Specifies the name of the package manager as well as a the command line arguments  to pass to the package manager when
installing, upgrading, and uninstalling packages.

| name | type | purpose |
| ---- | ---- | ------- |
| `manager.name` | String |  the name of the command or path to the executable to run |
| `manager.upgrade_args` | Array | an array of arguments to pass to the package manager for upgrading |
| `manager.install_args` | Array | an array of arguments to pass to the package manager for installation |
| `manager.un_install_args` | Array | an array of arguments to pass to the package manager for uninstallation |
| `manager.packages` | Array | an array of the names of packages to install |

## Packaging
To package all the target configuration and other files, create or edit a rconf configuration file. By default rconf
looks in `$HOME/.config/.rconf`, but can use any file provided to the `--file` argument. The resulting archive should
**always** have a `.tar` extension, if it is not provided it will be appended. For example, `rconf archive new_archive`
will produce an archive called `new_archive.tar`. The archive title may also be an absolute or relative path.

## Deployment
Deploying can be done in one of 2 ways.

The simplest method is running `rconf install archive.tar`. Rconf will unpack the archive, install the necessary
packages via the package manager and install the configuration files.

The other method is by unpacking the archive and running the packages `install.sh` to install the packages and
configurations for you. This method is especially helpful on systems without rconf.