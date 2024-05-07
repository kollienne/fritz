# Fritz 🐎

Fritz takes some of the hassle out of home-manager configuration.

If you like having your packages in a nice reproducable home.nix file, but miss being able to `apt search` or `dnf install`, you may find fritz mildly useful.

Fritz is currently more-or-less usable for simple things, but makes a lot of assumptions and is *largely untested*.

Currently Implemented:
- [x] Search
- [x] Cache nixpkgs index
- [x] Add package(s) to config
- [x] Remove package(s) from config
- [x] Git commit & push config
- [x] Configurable behaviour
- [ ] Simplify setup.

## Setup

You *can* use Fritz to directly modify `home.nix` (or any other .nix file), but the safer option would be to split the Fritz package list from the rest of your configuration. To do so add the following to your `home.nix` configuration.

```{nix}
imports = [
./fritz/packages.nix
];
```

and create a minimal `fritz/packages.nix` file:

```{nix}
{ config, pkgs, ... }:
{
  home.packages = [ 
  ];
}
```

Optionally, make `packages.nix` part of a .git repo.

## Configuration

By default Fritz looks for the configuration file in `~/.config/fritz/config.toml`. Use the command-line option `--config-file /path/to/some/other/file.toml` to load another file instead.
Options can also be set with environment variables, which take priority over the config file.

| Option                | Environment Variable         | Description                                                                                                                   |
|-----------------------|------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| package\_config\_file | FRITZ\_PACKAGE\_CONFIG\_FILE | Location of the .nix file Fritz will be adding/removing packages to/from. Default `~/.config/home-manager/fritz/packages.nix` |
| cache\_file\_path     | FRITZ\_CACHE\_FILE\_PATH     | Location in which to store the nixpkgs index cache. Default `~/.config/fritz/nixpkgs_cache.msgpack`                           |
| max\_cache\_age       | FRITZ\_MAX\_CACHE\_AGE       | Maximum age of the nixpkgs index cache before the index will be fetched again. Default `12h`                                  |
| num\_search\_results  | FRITZ\_NUM\_SEARCH\_RESULTS  | Maximum number of search results to print. Default 10.                                                                        |
| commit_change         | FRITZ_COMMIT_CHANGE          | Whether `config_file` changes will be commited (if `config_file` is in a .git repository. Default false.                      |
| push\_change          | FRITZ\_PUSH\_CHANGE          | Whether changes to `config_file` will result it git pushing the config file repo. Default false.                              |
| hm_switch             | FRITZ\_HM\_SWITCH            | Whether to run `home-manager switch` after changes to config file. Default true.                                              |
 
 
The default options will be used if no config file or environment variables are found.
`config.toml` is a sample config file.

## Usage

Usage: fritz [OPTIONS] <COMMAND>

Commands:
  add     
  rm      
  search  
  list    
  help    Print this message or the help of the given subcommand(s)

Options:
      --dry-run          
      --config <CONFIG>  
  -h, --help             Print help
  

| Command | Example                | Description                                                                                                                                                                                                            |
|---------|------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| add     | fritz add neovim emacs | Attempts to find full names for neovim (pkgs.neovim) and emacs (pkgs.emacs) in nixpkgs, then adds them to the config file. If configured, commits and pushes the changed config file, then runs `home-manager switch`. |
| rm      | fritz rm nano          | Removes nano (or pkgs.nano if found) from config file. If configured, commits and pushes config file, then runs `home-manager switch`.                                                                                 |
| search  | fritz search emacs gtk | Searches nixpkgs for packages containing *both* 'emacs' and 'gtk' in the package name and/or description. Results are weighted by the number of occurences of any search term.                                         |
| list    | fritz list             | Prints all packages currently in config file.                                                                                                                                                                          |


# Acknowledgements

.nix file parsing is borrowed more-or-less as is from https://github.com/nix-community/rnix-parser
