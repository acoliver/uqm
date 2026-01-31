/*
 * clock_rust.c - C wrappers that call Rust implementations of game clock functions
 *
 * This file provides C-callable wrappers for the Rust clock implementation.
 * When USE_RUST_CLOCK is enabled, these functions replace the C implementation
 * in clock.c. The wrappers call Rust extern functions via the staticlib.
 *
 * See rust/src/time/clock_bridge.rs for the Rust implementation.
 */

#include "clock.h"
#include "globdata.h"
#include "displist.h"
#include "gameev.h"
#include "libs/threadlib.h"
#include <stdlib.h>

// Mutex for clock access - matches C implementation
static Mutex clock_mutex;

// C implementations of helper functions copied from clock.c
// These are needed by the Rust code via extern "C"

//     every 4th year      but not 100s          yet still 400s
static BOOLEAN
IsLeapYear (COUNT year)
{
	return (year & 3) == 0 && ((year % 100) != 0 || (year % 400) == 0);
}

/* month is 1-based: 1=Jan, 2=Feb, etc. */
static BYTE
DaysInMonth (COUNT month, COUNT year)
{
	static const BYTE days_in_month[12] =
	{
		31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
	};

	if (month == 2 && IsLeapYear (year))
		return 29; /* February, leap year */

	return days_in_month[month - 1];
}

BOOLEAN
ValidateEvent (EVENT_TYPE type, COUNT *pmonth_index, COUNT *pday_index,
		COUNT *pyear_index)
{
	COUNT month_index, day_index, year_index;

	month_index = *pmonth_index;
	day_index = *pday_index;
	year_index = *pyear_index;
	if (type == RELATIVE_EVENT)
	{
		month_index += GLOBAL (GameClock.month_index) - 1;
		year_index += GLOBAL (GameClock.year_index) + (month_index / 12);
		month_index = (month_index % 12) + 1;

		day_index += GLOBAL (GameClock.day_index);
		while (day_index > DaysInMonth (month_index, year_index))
		{
			day_index -= DaysInMonth (month_index, year_index);
			if (++month_index > 12)
			{
				month_index = 1;
				++year_index;
			}
		}

		*pmonth_index = month_index;
		*pday_index = day_index;
		*pyear_index = year_index;
	}

	// translation: return (BOOLEAN) !(date < GLOBAL (Gameclock.date));
	return (BOOLEAN) (!(year_index < GLOBAL (GameClock.year_index)
			|| (year_index == GLOBAL (GameClock.year_index)
			&& (month_index < GLOBAL (GameClock.month_index)
			|| (month_index == GLOBAL (GameClock.month_index)
			&& day_index < GLOBAL (GameClock.day_index))))));
}

HEVENT
AddEvent (EVENT_TYPE type, COUNT month_index, COUNT day_index, COUNT
		year_index, BYTE func_index)
{
	extern void EventHandler (BYTE selector);
	HEVENT hNewEvent;

	if (type == RELATIVE_EVENT
			&& month_index == 0
			&& day_index == 0
			&& year_index == 0)
		EventHandler (func_index);
	else if (ValidateEvent (type, &month_index, &day_index, &year_index)
			&& (hNewEvent = AllocEvent ()))
	{
		EVENT *EventPtr;

		LockEvent (hNewEvent, &EventPtr);
		EventPtr->day_index = (BYTE)day_index;
		EventPtr->month_index = (BYTE)month_index;
		EventPtr->year_index = year_index;
		EventPtr->func_index = func_index;
		UnlockEvent (hNewEvent);

		{
			HEVENT hEvent, hSuccEvent;
			for (hEvent = GetHeadEvent (); hEvent != 0; hEvent = hSuccEvent)
			{
				LockEvent (hEvent, &EventPtr);
				if (year_index < EventPtr->year_index
						|| (year_index == EventPtr->year_index
						&& (month_index < EventPtr->month_index
						|| (month_index == EventPtr->month_index
						&& day_index < EventPtr->day_index))))
				{
					UnlockEvent (hEvent);
					break;
				}

				hSuccEvent = GetSuccEvent (EventPtr);
				UnlockEvent (hEvent);
			}
			
			InsertEvent (hNewEvent, hEvent);
		}

		return (hNewEvent);
	}

	return (0);
}

// Provide access to GameClock for Rust code
CLOCK_STATE *
GetGameClock (void)
{
	return &GLOBAL (GameClock);
}

// Declare the Rust extern functions
// These are implemented in rust/src/time/clock_bridge.rs
// and linked via the Rust staticlib

// Note: The functions have different names in clock_bridge.rs
// We need to match those exactly
extern int rust_clock_init(void);
extern int rust_clock_uninit(void);
extern void rust_clock_set_rate(int seconds_per_day);
extern void rust_clock_tick(void);
extern void rust_clock_advance_days(int days);
extern void rust_clock_lock(void);
extern void rust_clock_unlock(void);
extern int rust_clock_is_running(void);

// Initialize the game clock
BOOLEAN
InitGameClock (void)
{
	// Initialize the event queue - this is critical for save/load!
	if (!InitQueue (&GLOBAL (GameClock.event_q), NUM_EVENTS, sizeof (EVENT)))
		return FALSE;
	
	// Create the mutex for thread safety
	clock_mutex = CreateMutex ("Clock Mutex", SYNC_CLASS_TOPLEVEL);
	
	// Let Rust initialize the rest of the clock state
	return (BOOLEAN) rust_clock_init();
}

// Uninitialize the game clock
BOOLEAN
UninitGameClock (void)
{
	// Clean up the mutex
	DestroyMutex (clock_mutex);
	clock_mutex = NULL;
	
	// Clean up the event queue
	UninitQueue (&GLOBAL (GameClock.event_q));
	
	return (BOOLEAN) rust_clock_uninit();
}

// Set game clock rate (seconds per day)
void
SetGameClockRate (COUNT seconds_per_day)
{
	rust_clock_set_rate((int) seconds_per_day);
}

// Tick the game clock forward one tick
void
GameClockTick (void)
{
	rust_clock_tick();
}

// Move game clock forward by specific number of days
void
MoveGameClockDays (COUNT days)
{
	rust_clock_advance_days((int) days);
}

// Lock the game clock (for debugging)
void
LockGameClock (void)
{
	rust_clock_lock();
}

// Unlock the game clock (for debugging)
void
UnlockGameClock (void)
{
	rust_clock_unlock();
}

// Check if game clock is running
BOOLEAN
GameClockRunning (void)
{
	return (BOOLEAN) rust_clock_is_running();
}
