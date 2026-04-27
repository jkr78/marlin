def test_marlin_imports():
    import marlin
    assert marlin.__version__ == "0.1.0"


def test_core_extension_loads():
    from marlin import _core
    assert hasattr(_core, "__version__")
