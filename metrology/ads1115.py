import time

try:
    import smbus
except ImportError:
    smbus = None

from .adc_interface import ADCInterface

class ADS1115ADC(ADCInterface):
    """
    ADC implementation for ADS1115 using SMBus/I2C.
    """

    def __init__(self, bus=1, address=0x48, gain=1):
        """
        Initialize the ADS1115 ADC.

        Args:
            bus (int): I2C bus number.
            address (int): I2C address of the device.
            gain (int): Gain setting for ADS1115. Default 1.

        Raises:
            ImportError: If smbus is not installed.
        """
        if smbus is None:
            raise ImportError("smbus module is not installed. Please install it to use ADS1115ADC.")

        self.bus = smbus.SMBus(bus)
        self.address = address
        self.gain = gain

    def read_voltage(self, channel: int) -> float:
        """
        Reads the voltage from the specified channel (0-3).
        """
        if not 0 <= channel <= 3:
            raise ValueError("Channel must be between 0 and 3")

        # ADS1115 Config Register (0x01)
        # Bit 15: OS (1 to start conversion)
        # Bit 14-12: MUX (100 -> AIN0, 101 -> AIN1, 110 -> AIN2, 111 -> AIN3 for Single Ended)
        # Bit 11-9: PGA (Gain)
        # Bit 8: MODE (0 continuous, 1 single-shot)
        # Bit 7-5: DR (Data Rate)
        # Bit 4: COMP_MODE
        # Bit 3: COMP_POL
        # Bit 2: COMP_LAT
        # Bit 1-0: COMP_QUE (11 -> Disable comparator)

        # MUX calculation: 4 (100) + channel
        mux = 4 + channel

        config = 0x8000  # Start conversion
        config |= (mux << 12)
        config |= (1 << 9)  # PGA = 1 (Default +/- 4.096V)
        config |= (1 << 8)  # Single-shot mode
        config |= (4 << 5)  # Data rate 128 SPS
        config |= 0x03      # Disable comparator

        # Write config to register 0x01
        # Convert to bytes (MSB first)
        high = (config >> 8) & 0xFF
        low = config & 0xFF
        self.bus.write_i2c_block_data(self.address, 0x01, [high, low])

        # Wait for conversion (1/128s = ~8ms)
        time.sleep(0.01)

        # Read conversion register (0x00)
        data = self.bus.read_i2c_block_data(self.address, 0x00, 2)
        val = (data[0] << 8) | data[1]

        if val > 32767:
            val -= 65536

        # Convert to voltage (assuming PGA=1 => +/- 4.096V)
        voltage = val * 4.096 / 32768.0
        return voltage

    def close(self):
        self.bus.close()
