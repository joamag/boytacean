install_rgbds() {
    version=${1:-"0.8.0"}
    curl -s -L -o rgbds.tar.xz https://github.com/gbdev/rgbds/releases/download/v$version/rgbds-$version-linux-x86_64.tar.xz
    mkdir -p rgbds
    mv rgbds.tar.xz rgbds/
    pushd rgbds > /dev/null
    tar -xf rgbds.tar.xz
    popd > /dev/null
    export PATH="$CI_PROJECT_DIR/rgbds:$PATH"
    rgbasm --version
}
