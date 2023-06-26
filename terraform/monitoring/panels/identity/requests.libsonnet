local grafana   = import '../../grafonnet-lib/grafana.libsonnet';
local defaults  = import '../../grafonnet-lib/defaults.libsonnet';

local panels    = grafana.panels;
local targets   = grafana.targets;

{
  new(ds, vars)::
    panels.timeseries(
      title       = 'Requests',
      datasource  = ds.prometheus,
    )
    .configure(defaults.configuration.timeseries.withUnit('reqps'))

    .addTarget(targets.prometheus(
      datasource  = ds.prometheus,
      expr        = 'sum(rate(identity_lookup_counter{}[$__rate_interval]))',
      refId       = "Requests",
    ))
}