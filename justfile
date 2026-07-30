rootdir := ''
prefix := '/usr'
clean := '0'
debug := '0'
vendor := '0'
target := if debug == '1' { 'debug' } else { 'release' }
vendor_args := if vendor == '1' { '--frozen --offline' } else { '' }
debug_args := if debug == '1' { '' } else { '--release' }
cargo_args := vendor_args + ' ' + debug_args
targetdir := env('CARGO_TARGET_DIR', 'target')
sharedir := rootdir + prefix + '/share'
iconsdir := sharedir + '/icons/hicolor'
prefixdir := prefix + '/bin'
bindir := rootdir + prefixdir
libdir := rootdir + prefix + '/lib'
# The dock's pinned entries and the toolkit defaults are not shipped from here: they are
# part of the theme and live in the wmde-themes package.
cosmic-applets-bin := prefixdir / 'wmde-applets'
# Settings reads these to build a page per applet; the path is part of the contract with
# third-party applets, which install their own schema into the same directory.
applet-settings-dir := sharedir + '/wmde/applet-settings'
metainfo := 'fun.wmde.Applets.metainfo.xml'
metainfo-src := 'data' / metainfo
metainfo-dst := clean(rootdir / prefix) / 'share' / 'metainfo' / metainfo

default: build-release

# Compiles with debug profile
build-debug *args:
    cargo build {{ args }}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compile with a vendored tarball
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

_link_applet name:
    ln -sf {{ cosmic-applets-bin }} {{ bindir }}/{{ name }}

_install_icons name:
    find {{ name }}/'data'/'icons' -type f -exec echo {} \; | rev | cut -d'/' -f-3 | rev | xargs -d '\n' -I {} install -Dm0644 {{ name }}/'data'/'icons'/{} {{ iconsdir }}/{}

_install_desktop path:
    install -Dm0644 {{ path }} {{ sharedir }}/applications/{{ file_name(path) }}

_install_bin name:
    install -Dm0755 {{ targetdir }}/{{ target }}/{{ name }} {{ bindir }}/{{ name }}

# `dir` is the source crate directory (upstream cosmic-* name, not renamed);
# `link` is the installed symlink/argv0 name (wmde-*). They intentionally diverge:
# only the ROOT binary is renamed to wmde-applets, member crate dirs keep upstream names.
_install_applet id dir link: (_install_icons dir) (_install_desktop 'target/xdgen/' + id + '.desktop') (_link_applet link)

_install_applet_schemas:
    install -Dm0644 -t {{ applet-settings-dir }} data/applet-settings/*.ron

_install_metainfo:
    install -Dm0644 {{ metainfo-src }} {{ metainfo-dst }}

_install_status_notifier_watcher:
    sed "s|@bindir@|{{ prefixdir }}|" cosmic-applet-status-area/data/dbus-1/fun.wmde.StatusNotifierWatcher.service.in > cosmic-applet-status-area/data/dbus-1/fun.wmde.StatusNotifierWatcher.service
    install -Dm0644 cosmic-applet-status-area/data/dbus-1/fun.wmde.StatusNotifierWatcher.service {{ sharedir }}/dbus-1/services/fun.wmde.StatusNotifierWatcher.service
    sed "s|@bindir@|{{ prefixdir }}|" cosmic-applet-status-area/data/fun.wmde.StatusNotifierWatcher.service.in > cosmic-applet-status-area/data/fun.wmde.StatusNotifierWatcher.service
    install -Dm0644 cosmic-applet-status-area/data/fun.wmde.StatusNotifierWatcher.service {{ libdir }}/systemd/user/fun.wmde.StatusNotifierWatcher.service

_install_secret_agent_policy:
    install -Dm0644 cosmic-applet-network/data/dbus-1/system.d/fun.wmde.Settings.Applet.NetworkManager.SecretAgent.conf {{ sharedir }}/dbus-1/system.d/fun.wmde.Settings.Applet.NetworkManager.SecretAgent.conf

# Installs files into the system
install: (_install_bin 'wmde-applets') (_link_applet 'wmde-panel-button') (_install_applet 'fun.wmde.AppList' 'cosmic-app-list' 'wmde-app-list') (_install_applet 'fun.wmde.AppletA11y' 'cosmic-applet-a11y' 'wmde-applet-a11y') (_install_applet 'fun.wmde.AppletAudio' 'cosmic-applet-audio' 'wmde-applet-audio') (_install_applet 'fun.wmde.AppletInputSources' 'cosmic-applet-input-sources' 'wmde-applet-input-sources') (_install_applet 'fun.wmde.AppletBattery' 'cosmic-applet-battery' 'wmde-applet-battery') (_install_applet 'fun.wmde.AppletBluetooth' 'cosmic-applet-bluetooth' 'wmde-applet-bluetooth') (_install_applet 'fun.wmde.AppletMinimize' 'cosmic-applet-minimize' 'wmde-applet-minimize') (_install_applet 'fun.wmde.AppletNetwork' 'cosmic-applet-network' 'wmde-applet-network') (_install_applet 'fun.wmde.AppletNotifications' 'cosmic-applet-notifications' 'wmde-applet-notifications') (_install_applet 'fun.wmde.AppletPower' 'cosmic-applet-power' 'wmde-applet-power') (_install_applet 'fun.wmde.AppletStatusArea' 'cosmic-applet-status-area' 'wmde-applet-status-area') (_install_applet 'fun.wmde.AppletTiling' 'cosmic-applet-tiling' 'wmde-applet-tiling') (_install_applet 'fun.wmde.AppletTime' 'cosmic-applet-time' 'wmde-applet-time') (_install_applet 'fun.wmde.AppletWeather' 'cosmic-applet-weather' 'wmde-applet-weather') (_link_applet 'wmde-weather-settings') (_install_applet 'fun.wmde.AppletWorkspaces' 'cosmic-applet-workspaces' 'wmde-applet-workspaces') _install_applet_schemas _install_metainfo _install_status_notifier_watcher _install_secret_agent_policy

# Vendor Cargo dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor --locked | head -n -1 > .cargo/config
    echo 'directory = "vendor"' >> .cargo/config
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
[private]
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Bump cargo version, create git commit, and create tag
tag version:
    find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{ version }}"/' '{}' \; -exec git add '{}' \;
    cargo check
    cargo clean
    dch -D noble -v {{ version }}
    git add Cargo.lock debian/changelog
