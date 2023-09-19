use std::{collections::HashMap, path::PathBuf};

use solang_parser::pt::{self, CodeLocation, SourceUnit};

use crate::{
    engine::{EngineError, Outcome, Pushable},
    extractors::{primitive::EventExtractor, Extractor},
};

use super::{EventIndexing, OptimizationOutcome, OptimizationPattern};

impl OptimizationPattern for EventIndexing {
    fn find(source: &mut HashMap<PathBuf, SourceUnit>) -> Result<OptimizationOutcome, EngineError> {
        let mut outcome = Outcome::new();
        for (path_buf, source_unit) in source {
            let events = EventExtractor::extract(source_unit)?;

            //Accumulate the number of indexed events, and the number of non-array indexed parameters.
            for event in events.iter() {
                let mut indexed_events_count = 0;
                let mut non_array_indexed_parameter_count = 0;
                let event_parameters = event.fields.clone();
                for event_parameter in event_parameters.iter() {
                    if event_parameter.indexed {
                        indexed_events_count += 1;
                    }

                    if !matches!(event_parameter.ty, pt::Expression::ArraySlice(..)) {
                        non_array_indexed_parameter_count += 1;
                    }
                }
                //If there are more than 3 non-array indexed parameters, and less than 3 indexed events, then the event is not optimized.
                if non_array_indexed_parameter_count >= 3 && indexed_events_count < 3 {
                    outcome.push_or_insert(path_buf.clone(), event.loc(), event.to_string());

                //If there are less than 3 non-array indexed parameters, and the number of indexed events is not equal to the number of non-array indexed parameters, then the event is not optimized.
                } else if non_array_indexed_parameter_count < 3
                    && indexed_events_count != non_array_indexed_parameter_count
                {
                    outcome.push_or_insert(path_buf.clone(), event.loc(), event.to_string());
                }
            }
        }
        Ok(OptimizationOutcome::EventIndexing(outcome))
    }
}
mod test {
    use std::{fs::File, io::Write};

    use crate::{
        optimizations::{EventIndexing, OptimizationPattern},
        report::ReportSectionFragment,
        utils::MockSource,
    };

    #[test]
    fn test_immutable_variables_optimization() -> eyre::Result<()> {
        let file_contents = r#"
    
    pragma solidity >= 0.8.0;
    contract Contract {

        event IsNotOptimized(address addr1, address indexed addr2);
        event IsOptimized(address indexed addr1, address indexed addr2, address indexed addr3);
        event AlsoIsNotOptimized(address addr1, address indexed addr2, address indexed addr3);
        
    }
 
    "#;
        let mut source = MockSource::new().add_source("event_indexing.sol", file_contents);
        let optimization_locations = EventIndexing::find(&mut source.source)?;
        assert_eq!(optimization_locations.len(), 2);
        let report: Option<ReportSectionFragment> = optimization_locations.into();
        if let Some(report) = report {
            let mut f = File::options()
                .append(true)
                .open("mocks/optimization_report_sections.md")?;
            writeln!(&mut f, "{}", &String::from(report))?;
        }
        Ok(())
    }
}