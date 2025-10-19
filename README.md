# testeq-rs

This crate provides abstraction and support for controlling test equipment.

Currently under development, and is not yet in a very usable or stable state.

## Supported transports

* SCPI
  * SCPI over VXI-11 (TCP)
    * Recommended
  * SCPI over raw TCP
  * SCPI over serial port

## Supported test equipment

* Power supplies
  * Rigol DP700, DP800, DP900, DP2000 series
    * Only tested on DP832
  * Siglent SPD1000X, SPD3000, SPD4000X series
    * Only tested on SPD4306X
* Multimeters
  * Siglent SDM4065A
    * Currently only minimal support
* Oscilloscopes
  * Siglent SDS3000X HD
    * Currently only minimal support
    * Only SDS3104X HD supported currently
    * Only works fully over VXI-11 transport
* Spectrum Analyzers
  * Siglent SSA3000X Plus
    * Currently only minimal support
    * Only SSA3075X Plus tested, others likely to work
    * Other SSA series devices may work as well
* AC power sources
  * HP/Agilent/Keysight 6800
    * Currently only minimal readback support
    * Only 6811B tested, others likely to work for phase 1
