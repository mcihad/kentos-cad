# Mekânsal istatistik R'la (docs/adr/0639): fpc::clusterboot on the runs spatial_r_cases.py writes (§1), and
# spatstat's clarkevans with Donnelly's correction on its windows (§2).
#
#   Rscript scripts/fixtures/spatial_r.R IN.json OUT.json
#
# IN holds the places and, per run, the clustering (DBSCAN's radius and least points, or k-means' k), the resampling
# (boot or subset) and the resamples' index lists KentOS's rule draws. clusterboot draws with R's `sample`; a copy of it
# whose `sample` hands out those lists makes it cluster the same resamples, so its bootmean, bootrecover and bootbrd are
# comparable value for value. The partitions are R's own: dbscan::dbscan (the standard DBSCAN; fpc's own dbscan is a
# variant) and stats::kmeans (Lloyd, one start) from the farthest-first seeds written here from ADR 0238 §10.

suppressPackageStartupMessages({
  library(jsonlite)
  library(fpc)
  library(dbscan)
  library(spatstat.explore)
})

args <- commandArgs(trailingOnly = TRUE)
input <- fromJSON(args[1], simplifyVector = FALSE)
pts <- do.call(rbind, lapply(input$points, function(p) c(p[[1]], p[[2]])))

# clusterboot's draws, one list after another.
lists <- NULL
drawn <- 0
boot <- fpc::clusterboot
hooks <- new.env(parent = environment(boot))
hooks$sample <- function(...) {
  drawn <<- drawn + 1
  lists[[drawn]]
}
environment(boot) <- hooks

# A clusterboot method's answer: the clusters as logical columns, noise the last one (noisemethod). A resample without
# clusters says no noise either: clusterboot's noise branch indexes with the `ncases` of the last resample that had
# clusters, longer than this one's (only the noise's own γ, which nothing compares).
answer <- function(partition) {
  k <- max(c(0, partition))
  clusters <- lapply(seq_len(k), function(c) partition == c)
  noise <- k > 0 && any(partition == 0)
  if (noise) clusters[[k + 1]] <- partition == 0
  list(result = partition, nc = k + noise, nccl = k, clusterlist = clusters, partition = partition,
       clustermethod = "KentOS")
}

dbscan_cbi <- function(data, eps, minPts, borderNoise) {
  answer(dbscan::dbscan(data, eps = eps, minPts = minPts, borderPoints = !borderNoise)$cluster)
}

# ADR 0238 §10's seeds: the place nearest the mean, then each time the place farthest from the chosen ones (ties to the
# smaller index), then Lloyd's rounds (at most 500).
seeds <- function(x, k) {
  d2 <- function(p) rowSums((x - matrix(p, nrow(x), 2, byrow = TRUE))^2)
  chosen <- which.min(d2(colMeans(x)))
  least <- d2(x[chosen, ])
  while (length(chosen) < k) {
    far <- which.max(least)
    chosen <- c(chosen, far)
    least <- pmin(least, d2(x[far, ]))
  }
  x[chosen, , drop = FALSE]
}

kmeans_cbi <- function(data, k) {
  k <- min(k, nrow(unique(data)))
  fit <- stats::kmeans(data, centers = seeds(data, k), iter.max = 500, algorithm = "Lloyd")
  answer(fit$cluster)
}

out <- list()
for (run in input$runs) {
  lists <- lapply(run$lists, function(l) as.integer(unlist(l)) + 1L)
  drawn <- 0
  method <- if (run$kind == "dbscan") dbscan_cbi else kmeans_cbi
  extra <- if (run$kind == "dbscan") {
    list(eps = run$radius, minPts = run$minPoints, borderNoise = isTRUE(run$borderNoise))
  } else {
    list(k = run$k)
  }
  fit <- do.call(boot, c(list(data = pts, B = length(lists), bootmethod = run$scheme, clustermethod = method,
                              noisemethod = TRUE, count = FALSE, subtuning = floor(nrow(pts) / 2)), extra))
  stopifnot(drawn == length(lists))
  k <- fit$result$nccl
  result <- if (run$scheme == "boot") fit$bootresult else fit$subsetresult
  out[[run$id]] <- list(
    partition = fit$partition,
    means = if (run$scheme == "boot") fit$bootmean[seq_len(k)] else fit$subsetmean[seq_len(k)],
    recovered = if (run$scheme == "boot") fit$bootrecover[seq_len(k)] else fit$subsetrecover[seq_len(k)],
    dissolved = if (run$scheme == "boot") fit$bootbrd[seq_len(k)] else fit$subsetbrd[seq_len(k)],
    gammas = lapply(seq_len(k), function(c) result[c, ])
  )
}
# Clark and Evans with Donnelly's correction on a rectangle: the observed mean distance, the window's area and perimeter,
# spatstat's expectation and R (its coefficient 0.0412 and Clark and Evans's error, as its source has them).
edges <- list()
for (w in input$donnelly) {
  xy <- do.call(rbind, lapply(w$points, function(p) c(p[[1]], p[[2]])))
  win <- owin(c(w$x0, w$x1), c(w$y0, w$y1))
  X <- ppp(xy[, 1], xy[, 2], window = win)
  ce <- spatstat.explore:::clarkevansCalc(X, correction = "Donnelly", working = TRUE)
  work <- attr(ce, "working")
  edges[[w$id]] <- list(observed = work$Dobs, area = work$areaW, perimeter = work$perim, expected = work$Dkevin,
                        ratio = unname(ce[["Donnelly"]]))
}
out[["donnelly"]] <- edges
writeLines(toJSON(out, digits = NA, auto_unbox = FALSE), args[2])
