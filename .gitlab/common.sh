install_rgbds() {
    version=${1:-"0.8.0"}
    curl -L -o rgbds.tar.xz https://github.com/gbdev/rgbds/releases/download/v$version/rgbds-$version-linux-x86_64.tar.xz
    mkdir -p rgbds
    mv rgbds.tar.xz rgbds/
    pushd rgbds
    tar -xf rgbds.tar.xz
    popd
    export PATH="$CI_PROJECT_DIR/rgbds:$PATH"
    rgbasm --version
}
