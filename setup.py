import os
import sys

try:
    import setuptools
except ImportError:
    import subprocess

    errno = subprocess.call([sys.executable, "-m", "pip", "install", "setuptools"])
    if errno:
        print("Please install setuptools package")
        raise SystemExit(errno)
    else:
        import setuptools

try:
    import setuptools_rust
except ImportError:
    import subprocess

    errno = subprocess.call([sys.executable, "-m", "pip", "install", "setuptools-rust"])
    if errno:
        print("Please install setuptools-rust package")
        raise SystemExit(errno)
    else:
        import setuptools_rust

setuptools.setup(
    name="boytacean",
    version="0.10.5",
    author="João Magalhães",
    author_email="joamag@gmail.com",
    description="A Game Boy emulator that is written in Rust",
    license="Apache License, Version 2.0",
    keywords="gameboy emulator rust",
    url="https://boytacean.joao.me",
    packages=["boytacean"],
    test_suite="boytacean.test",
    package_dir={"": os.path.normpath("src/python")},
    rust_extensions=[
        setuptools_rust.RustExtension(
            "boytacean.boytacean",
            binding=setuptools_rust.Binding.PyO3,
            features=["python"],
        )
    ],
    install_requires=[],
    setup_requires=["setuptools-rust", "wheel", "pillow"],
    include_package_data=True,
    zip_safe=False,
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Topic :: Utilities",
        "License :: OSI Approved :: Apache Software License",
        "Operating System :: OS Independent",
        "Programming Language :: Python",
        "Programming Language :: Python :: 3.5",
        "Programming Language :: Python :: 3.6",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
    ],
    long_description=open(os.path.join(os.path.dirname(__file__), "README.md"), "rb")
    .read()
    .decode("utf-8"),
    long_description_content_type="text/markdown",
)
