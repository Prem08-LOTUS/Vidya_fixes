from abc import ABC, abstractmethod

class ADCInterface(ABC):
    """
    Abstract base class for ADC implementations.
    """

    @abstractmethod
    def read_voltage(self, channel: int) -> float:
        """
        Reads the voltage from the specified channel.

        Args:
            channel (int): The ADC channel to read from.

        Returns:
            float: The voltage read in Volts.
        """
        pass
