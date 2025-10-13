# testeq-rs

This crate provides abstraction and support for controlling test equipment.

Currently under development, and is not yet in a very usable or stable state.

## Supported transports

* SCPI
  * SCPI over raw TCP
  * SCPI over VXI-11 (TCP)

## Supported test equipment

* Power supplies
  * Rigol DP800
    * Only DP832 supported currently
* Multimeters
  * Siglent SDM4065A
    * Currently only minimal support
* Oscilloscopes
  * Siglent SDS3000X HD
    * Currently only minimal support
    * Only SDS3104X HD supported currently
* Spectrum Analyzers
  * Siglent SSA3000X Plus
    * Currently only minimal support
    * Only SSA3075X Plus tested, others likely to work
    * Other SSA series devices may work as well
