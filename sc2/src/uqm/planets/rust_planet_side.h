// Transitional C declaration for the Rust-owned PlanetSide dispatcher.
#ifndef UQM_PLANETS_RUST_PLANET_SIDE_H_
#define UQM_PLANETS_RUST_PLANET_SIDE_H_

#include "libs/compiler.h"

#include <stdint.h>

#define RUST_PLANET_SIDE_ABI_VERSION 2u
#define RUST_PLANET_SIDE_OP_RUN_SESSION 3u
#define RUST_PLANET_SIDE_STATUS_OK 0
#define RUST_PLANET_SIDE_DETAIL_ADAPTER_FAILURE 4u
#define RUST_PLANET_SIDE_DETAIL_FRAME_BUDGET 5u

typedef struct RustPlanetSideRunContext
{
	void *solar_system;
	void *world;
	void *misc_data_frame;
	void *energy_frame;
	void *life_frames[3];
	int32_t landing_x;
	int32_t landing_y;
	uint8_t facing;
	uint8_t padding[3];
	uint32_t retrieval_masks[3];
	uint32_t tick_period;
	uint32_t frame_budget;
} RustPlanetSideRunContext;

typedef struct RustPlanetSideRequest
{
	uint32_t abi_version;
	uint32_t operation;
	int32_t argument0;
	int32_t argument1;
	void *context;
} RustPlanetSideRequest;

typedef struct RustPlanetSideReply
{
	int32_t status;
	uint32_t detail;
	int64_t value0;
	int64_t value1;
} RustPlanetSideReply;

int32_t uqm_rust_planet_side (const RustPlanetSideRequest *request,
		RustPlanetSideReply *reply);

#endif /* UQM_PLANETS_RUST_PLANET_SIDE_H_ */
