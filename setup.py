from os.path import dirname, join, normpath
from sys import executable

try:
    from setuptools import find_packages, setup
except ImportError:
    from subprocess import call

    errno = call([executable, "-m", "pip", "install", "setuptools"])
    if errno:
        print("Please install setuptools package")
        raise SystemExit(errno)
    else:
        from setuptools import find_packages, setup

try:
    from setuptools_rust import Binding, RustExtension
except ImportError:
    from subprocess import call

    errno = call([executable, "-m", "pip", "install", "setuptools-rust"])
    if errno:
        print("Please install setuptools-rust package")
        raise SystemExit(errno)
    else:
        from setuptools_rust import Binding, RustExtension

setup(
    name="boytacean",
    version="0.15.0",
    author="João Magalhães",
    author_email="joamag@gmail.com",
    description="A Game Boy emulator that is written in Rust",
    license="Apache License, Version 2.0",
    keywords="gameboy emulator rust",
    url="https://boytacean.joao.me",
    packages=find_packages(where=normpath("src/python")),
    test_suite="boytacean.test",
    package_dir={"": normpath("src/python")},
    rust_extensions=[
        RustExtension(
            "boytacean.boytacean",
            binding=Binding.PyO3,
            features=["python"],
            debug=False,
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
    long_description=open(join(dirname(__file__), "README.md"), "rb")
    .read()
    .decode("utf-8"),
    long_description_content_type="text/markdown",
)
