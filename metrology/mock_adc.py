import random
from .adc_interface import ADCInterface

class MockADC(ADCInterface):
    """
    Mock ADC implementation for testing and simulation.
    """

    def __init__(self, base_voltage=1.65, noise_amplitude=0.01):
        """
        Initialize the Mock ADC.

        Args:
            base_voltage (float): The base voltage to return.
            noise_amplitude (float): Amplitude of random noise added.
        """
        self.base_voltage = base_voltage
        self.noise_amplitude = noise_amplitude

    def read_voltage(self, channel: int) -> float:
        """
        Returns a simulated voltage with noise.
        """
        noise = random.uniform(-self.noise_amplitude, self.noise_amplitude)
        return self.base_voltage + noise
