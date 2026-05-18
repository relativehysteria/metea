* Write some functions for changing the state and automatically writing it to
  storage
* Add floods/air quality.
* Custom coordinates instead of using the geocoding API, as well as country code
  for the geocoding API.
* Better graph for cloud cover (I'd like them in one place, but the current
  graph is not great)
* Add keyboard navigation on desktop.
* Add compare mode for
  - multiple locations
  - multiple models
* Store source model + run timestamp (important when for example ICON updated
  but GFS didn't)
* Start using actual versioning scheme.
* Switch to xbuild/apk2.
* Figure out why on android we don't come back from app hibernation.
* Cache. Reload data after one hour, or on app start.
* Figure out HTTPS.
* Make writing to wrong files inexpressible.
* Figure out a better way to handle font other than writing heading()
  everywhere.
* Document all the fuckery; why contexts are needed in queries, etc.
* Handle drag scroll on mobile; currently it only works when dragging inside
  the graphs (which also works on desktop)
* Implement settings
  - Global settings (e.g. what graphs to show as well as their order)
  - Per-location settings (same as global settings but overwrite for a
    specific location, as well as stuff like panel tilt and azimuth for GTI,
    which doesn't make much sense to be made global)
  - Units: temperature, wind speed, precipitation
  - Colorschemes.
* Figure out graph interactions:
  - Add tooltips or explanations. Should possibly be an option, or be at the
     bottom of the page.
  - Toggle viewing variables when graphs are clicked.
  - Show specific value of variables at X coord when clicked.
  - Synchronize hover on all graphs (i.e. if one point is selected, it is
    selected on all graphs)
  - Add min/max markers.
