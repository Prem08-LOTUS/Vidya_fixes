import logging
from typing import Optional

from .adc_interface import ADCInterface
from .mock_adc import MockADC
from .mcp3008 import MCP3008ADC
from .ads1115 import ADS1115ADC

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

class TemperatureController:
    """
    Controller for reading temperature from a thermistor via an ADC.
    """

    def __init__(self, adc_type: str = "mock", adc_channel: int = 0, adc_options: Optional[dict] = None):
        """
        Initialize the TemperatureController.

        Args:
            adc_type (str): Type of ADC to use ("mock", "mcp3008", "ads1115").
            adc_channel (int): The ADC channel where the thermistor is connected.
            adc_options (dict, optional): Additional options for ADC initialization.
        """
        self.adc_channel = adc_channel
        self.adc: ADCInterface

        if adc_options is None:
            adc_options = {}

        if adc_type == "mock":
            logger.info("Initializing Mock ADC")
            self.adc = MockADC(**adc_options)
        elif adc_type == "mcp3008":
            try:
                logger.info("Initializing MCP3008 ADC")
                self.adc = MCP3008ADC(**adc_options)
            except ImportError as e:
                logger.error(f"Failed to initialize MCP3008: {e}. Falling back to Mock.")
                self.adc = MockADC()
        elif adc_type == "ads1115":
            try:
                logger.info("Initializing ADS1115 ADC")
                self.adc = ADS1115ADC(**adc_options)
            except ImportError as e:
                logger.error(f"Failed to initialize ADS1115: {e}. Falling back to Mock.")
                self.adc = MockADC()
        else:
            raise ValueError(f"Unknown ADC type: {adc_type}")

    def read_thermistor_voltage(self) -> float:
        """
        Read thermistor voltage from the configured ADC.
        """
        try:
            voltage = self.adc.read_voltage(self.adc_channel)
            return voltage
        except Exception as e:
            logger.error(f"Error reading voltage: {e}")
            return 0.0

    def voltage_to_temperature(self, voltage: float) -> float:
        """Convert voltage to temperature"""
        # Linear approximation around 25C?
        # Original code: return 25.0 - (voltage - 1.65) / 0.33
        return 25.0 - (voltage - 1.65) / 0.33

    def get_temperature(self) -> float:
        """
        Get the current temperature in Celsius.
        """
        voltage = self.read_thermistor_voltage()
        return self.voltage_to_temperature(voltage)
