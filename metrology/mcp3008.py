try:
    import spidev
except ImportError:
    spidev = None

from .adc_interface import ADCInterface

class MCP3008ADC(ADCInterface):
    """
    ADC implementation for MCP3008 using SPI.
    """

    def __init__(self, bus=0, device=0, vref=3.3):
        """
        Initialize the MCP3008 ADC.

        Args:
            bus (int): SPI bus number.
            device (int): SPI device number.
            vref (float): Reference voltage (usually 3.3V or 5.0V).

        Raises:
            ImportError: If spidev is not installed.
        """
        if spidev is None:
            raise ImportError("spidev module is not installed. Please install it to use MCP3008ADC.")

        self.spi = spidev.SpiDev()
        self.spi.open(bus, device)
        self.spi.max_speed_hz = 1350000
        self.vref = vref

    def read_voltage(self, channel: int) -> float:
        """
        Reads the voltage from the specified channel (0-7).
        """
        if not 0 <= channel <= 7:
            raise ValueError("Channel must be between 0 and 7")

        # MCP3008 protocol:
        # Start bit, Single/Diff (1), D2, D1, D0
        # 0000 0001 (Start) -> 1
        # 1000 0000 (SGL/DIFF=1, D2=channel>>2, D1=(channel>>1)&1, D0=channel&1) << 4

        cmd = 128 # 1000 0000
        cmd |= ((channel & 0x07) << 4)

        # Send 1, cmd, 0
        resp = self.spi.xfer2([1, cmd, 0])

        # Result is 10 bits
        # resp[1] & 3 -> last 2 bits
        # resp[2] -> last 8 bits
        result = ((resp[1] & 3) << 8) + resp[2]

        voltage = (result * self.vref) / 1023.0
        return voltage

    def close(self):
        self.spi.close()
