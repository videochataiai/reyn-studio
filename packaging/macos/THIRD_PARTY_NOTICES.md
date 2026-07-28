# Reyn Studio macOS Python Runtime Notices

This bundle includes Reyn Studio's lightweight Python source closure, but it does
not redistribute a Python interpreter, Python wheels, native Python extensions,
PyTorch, model weights, detached model signatures, or TUF repository metadata.
The components below are external runtime requirements recorded from
`reyn-research/uv.lock`; their licenses apply when an operator supplies them.

## cryptography 49.0.0

Project: <https://github.com/pyca/cryptography>

License: Apache License 2.0 OR BSD 3-Clause License.

Copyright (c) individual contributors to the `cryptography` project.

## safetensors 0.8.0

Project: <https://github.com/huggingface/safetensors>

License: Apache License 2.0.

Copyright (c) Hugging Face and contributors.

## python-tuf 7.0.0

Project: <https://github.com/theupdateframework/python-tuf>

License: MIT License.

Copyright (c) The Update Framework contributors.

## securesystemslib 1.4.0

Project: <https://github.com/secure-systems-lab/securesystemslib>

License: MIT License.

Copyright (c) New York University and the securesystemslib contributors.

## NumPy 2.5.0

Project: <https://numpy.org/>

License: BSD 3-Clause License.

Copyright (c) 2005-2026, NumPy Developers.

NumPy wheels contain additional third-party components and notices. An operator
redistributing a Python environment must preserve the complete license inventory
from that environment rather than treating this summary as a wheel license file.

## PyTorch 2.12.1

Project: <https://pytorch.org/>

License: BSD 3-Clause License.

Copyright (c) 2016-2026 Facebook, Inc. and PyTorch contributors.

PyTorch distributions include a separate `NOTICE` and licenses for bundled
third-party components. An operator redistributing PyTorch must preserve those
distribution files; they are not copied here because PyTorch is not bundled.

## Python 3.14 or newer

Project: <https://www.python.org/>

License: Python Software Foundation License Version 2 and applicable historical
licenses.

Copyright (c) 2001-2026 Python Software Foundation; all rights reserved.

The full license texts accompany the upstream distributions linked above. This
notice is an inventory and attribution record, not a replacement for license
files required when those external distributions are redistributed.
