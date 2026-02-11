import unittest
import sys
import logging
from unittest.mock import MagicMock, patch

# Adjust sys.path to include the root directory
sys.path.append('.')

from metrology.temperature_controller import TemperatureController
from metrology.mock_adc import MockADC
from metrology.mcp3008 import MCP3008ADC
from metrology.ads1115 import ADS1115ADC

class TestTemperatureController(unittest.TestCase):

    def test_mock_adc(self):
        """Test TemperatureController with MockADC."""
        # Initialize controller with Mock ADC
        controller = TemperatureController(adc_type="mock", adc_options={"base_voltage": 1.65, "noise_amplitude": 0.0})

        # Read voltage
        voltage = controller.read_thermistor_voltage()
        self.assertAlmostEqual(voltage, 1.65)

        # Read temperature
        temp = controller.get_temperature()
        # 25.0 - (1.65 - 1.65) / 0.33 = 25.0
        self.assertAlmostEqual(temp, 25.0)

    def test_mock_adc_with_noise(self):
        """Test TemperatureController with MockADC and noise."""
        controller = TemperatureController(adc_type="mock", adc_options={"base_voltage": 1.65, "noise_amplitude": 0.1})
        voltage = controller.read_thermistor_voltage()
        self.assertTrue(1.55 <= voltage <= 1.75)

    def test_voltage_to_temperature(self):
        """Test voltage to temperature conversion logic."""
        controller = TemperatureController(adc_type="mock")

        # Test 1.65V -> 25C
        self.assertAlmostEqual(controller.voltage_to_temperature(1.65), 25.0)

        # Test 1.98V -> 25 - (1.98 - 1.65)/0.33 = 25 - 0.33/0.33 = 24.0
        self.assertAlmostEqual(controller.voltage_to_temperature(1.98), 24.0)

        # Test 1.32V -> 25 - (1.32 - 1.65)/0.33 = 25 - (-0.33)/0.33 = 26.0
        self.assertAlmostEqual(controller.voltage_to_temperature(1.32), 26.0)

    def test_mcp3008_fallback(self):
        """Test that MCP3008 falls back to MockADC if missing."""
        # Ensure spidev is not importable (mocking import error)
        with patch.dict('sys.modules', {'spidev': None}):
            # We also need to patch the MCP3008ADC class to raise ImportError on init
            # OR rely on the fact that mcp3008.py sets spidev=None and raises ImportError

            # Since we modify sys.modules, reloading mcp3008 might be tricky.
            # Instead, let's just instantiate with "mcp3008" and assert it falls back because spidev is missing in this env.

            # Verify spidev is missing in this env
            try:
                import spidev
            except ImportError:
                controller = TemperatureController(adc_type="mcp3008")
                self.assertIsInstance(controller.adc, MockADC)
                return

            # If spidev IS installed (unlikely), force the error
            with patch('metrology.mcp3008.spidev', None):
                 controller = TemperatureController(adc_type="mcp3008")
                 self.assertIsInstance(controller.adc, MockADC)

    def test_ads1115_fallback(self):
        """Test that ADS1115 falls back to MockADC if missing."""
        try:
            import smbus
        except ImportError:
            controller = TemperatureController(adc_type="ads1115")
            self.assertIsInstance(controller.adc, MockADC)
            return

        with patch('metrology.ads1115.smbus', None):
             controller = TemperatureController(adc_type="ads1115")
             self.assertIsInstance(controller.adc, MockADC)

if __name__ == '__main__':
    unittest.main()
