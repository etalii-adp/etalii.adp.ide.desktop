# Recent graduates

Recent US college graduates, under 28, surveyed in 2010 to 2012, by the category of their major: how many were working, unemployed or not in the labour force, and of those working, whether their job needed a college degree. In thousands of people.

## Source

**The data is FiveThirtyEight's [`college-majors/recent-grads.csv`](https://github.com/fivethirtyeight/data/tree/master/college-majors)**, vendored on 2026-10-04, which FiveThirtyEight derived from the American Community Survey 2010-2012 Public Use Microdata Series for the article *The Economic Guide To Picking A College Major*. The data repository's licence is Creative Commons Attribution 4.0 International, vendored verbatim beside this file as `LICENSE.md`. The changes below were made by this repository; FiveThirtyEight does not endorse them.

**How the figures were derived.** The 172 majors were summed into their 16 major categories (`Major_category`). One major, Food Science, has no total in the file and is left out. For each category, *Not in the labour force* is `Total − Employed − Unemployed`. From the employed, *Job requiring a degree* is `College_jobs`, *Job not requiring a degree* is `Non_college_jobs`, and *Job not classified* is what is left of `Employed`, because the survey classified fewer jobs than it counted people in work. Every figure is in thousands, rounded to one decimal.

## What it shows

| Part | In this example |
|---|---|
| Nodes | 22, in three columns |
| Flows | 51: 16 categories each into three outcomes, and the employed into three kinds of job |
| Colour by target | Every flow takes the colour of the outcome it reaches (`flow-color: target`), so the share of each category that works reads at a glance |
| Value format | `{value}k` |

## What it does not demonstrate

- **Individual majors.** The 172 majors are summed into categories; the diagram would be legible with them, but not on one screen.
- **Earnings.** The file's median earnings are not a quantity that flows, so they are left out.
- **Notes, custom colours or stated columns.**
